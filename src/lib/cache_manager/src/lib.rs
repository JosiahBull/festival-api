use config::Config;
use priority_queue::PriorityQueue;
use rocket::{
    async_trait, debug, error,
    fairing::AdHoc,
    futures::future::join_all,
    request::FromRequest,
    tokio::{
        self,
        fs::remove_file,
        runtime::Runtime,
        sync::{
            mpsc::{self},
            RwLock,
        },
    },
    warn,
};
use std::{
    collections::HashSet,
    convert::{Infallible, TryInto},
    io::ErrorKind,
    os::unix::prelude::OsStrExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
};

/// Handles the size and implementation of the cache folder for any application
/// automatically removing files as needed.
#[derive(Debug)]
pub struct CacheManager {
    cache: PriorityQueue<[u8; 32], (i32, u32)>, //Hash -> (priority, file_size) // This means that biggers files will get removed over smaller ones
    cache_path: PathBuf,
    temp_path: PathBuf,
    max_allowed_size_bytes: u64,
    current_size_bytes: u64,
    rx: mpsc::UnboundedReceiver<CacheAction>,
    restricted_files: HashSet<String>,
}

impl CacheManager {
    pub fn new(cache_path: PathBuf, temp_path: PathBuf, max_allowed_size_mb: u64) -> Result<(Self, mpsc::UnboundedSender<CacheAction>), Box<dyn std::error::Error>> {
        //Construct
        let (tx, rx) = mpsc::unbounded_channel();
        let mut res = CacheManager {
            cache_path,
            temp_path,
            max_allowed_size_bytes: max_allowed_size_mb * 1_000_000,
            current_size_bytes: 0,
            rx,
            cache: PriorityQueue::new(),
            restricted_files: vec![String::from(".gitkeep")].into_iter().collect(),
        };

        //Get current total size of files
        let paths = match std::fs::read_dir(&res.cache_path) {
            Ok(f) => f,
            Err(e) => {
                panic!("unable to read files in cache due to {}", e);
            }
        };

        for entry in paths {
            if let Ok(entry) = entry {
                error!("ENTRY_NAME: {:?}", entry.file_name()); //temp
                match entry.file_name().to_str() {
                    Some(f) if !res.restricted_files.contains(f) => {}
                    Some(_) => continue,
                    None => {
                        warn!("unable to process a file when initalising cache");
                        continue;
                    }
                }

                let md = match entry.metadata() {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("unable to scan cache file {}", e);
                        continue;
                    }
                };

                if md.is_dir() {
                    debug!("encountered directory in cache, skipping");
                    continue;
                }

                let hash_name: [u8; 32] = match entry.file_name().as_bytes().try_into() {
                    Ok(f) => f,
                    Err(_) => {
                        error!("found unexpected file in cache {:?}", entry.file_name());
                        continue;
                    }
                };
                res.cache.push(hash_name, (i32::MAX, md.len() as u32));
                res.current_size_bytes += md.len();
            }
        }
        Ok((res, tx))
    }

    /// A file has been used, increase it's probability of staying in the cache
    pub fn used(&mut self, hash: [u8; 32], size: u32) {
        self.current_size_bytes += size as u64;
        match self.cache.get(&hash).is_some() {
            true => self.cache.change_priority_by(&hash, |x| x.0 -= 1),
            false => {
                self.cache.push(hash, (i32::MAX, size));
            }
        }
    }

    /// If the cache is greater than the maximum allowed size, trims files in the cache
    pub async fn enforce_cache_size(&mut self) {
        //If greater than allowed, trim files
        if self.current_size_bytes > self.max_allowed_size_bytes {
            //Number of workers to carry out I/O Operations with
            const WORKER_COUNT: usize = 10;

            //Total size to remove, and size removed thus far
            let size_to_remove = self.current_size_bytes / 4;
            let size_removed_master = Arc::new(AtomicU64::new(0));

            //Items removed from our priority queue
            let removed_master = Arc::new(RwLock::new(PriorityQueue::new()));
            let header_master = Arc::new(RwLock::new(self));

            //Handles to our async threads carrying out i/o ops
            let mut handles = vec![];

            //Create our i/o thread pool, execute until we succeed or run out of items to remove
            for _ in 0..WORKER_COUNT {
                let size_removed = size_removed_master.clone();
                let header = header_master.clone();
                let removed = removed_master.clone();
                handles.push(async move {
                    while size_removed.load(Ordering::Relaxed) < size_to_remove {
                        if let Some((file_name, (priority, file_size))) =
                            header.write().await.cache.pop()
                        {
                            //Check if file exists, and if it does remove it
                            let file_name_string =
                                format!("{}.wav", String::from_utf8_lossy(&file_name));
                            let path = header.read().await.cache_path.join(&file_name_string);
                            let path = path.as_path();
                            match tokio::fs::metadata(path).await {
                                Ok(md) if md.is_file() => {
                                    match tokio::fs::remove_file(path).await {
                                        Ok(_) => {
                                            size_removed
                                                .fetch_add(file_size as u64, Ordering::Relaxed);
                                        }
                                        Err(e) => error!(
                                            "failed to remove cached file due to error {}",
                                            e
                                        ),
                                    }
                                }
                                Ok(_) => warn!(
                                    "attempted to remove file that was directory in cache {:?}",
                                    &path
                                ),
                                Err(e) => error!(
                                    "failed to read metadata of file {:?} error {}",
                                    &path, e
                                ),
                            }

                            //Only keep items that have at least 5 uses, items with a single use etc shoudln't be kept
                            //This helps to prevent excess memory usage through storage of priority 1 objects
                            if i32::MAX - priority > 5 {
                                removed.write().await.push(file_name, (priority, file_size));
                            }
                        } else {
                            break;
                        }
                    }
                });
            }
            join_all(handles).await;

            let mut self_ref = header_master.write().await;
            self_ref.current_size_bytes -= size_removed_master.load(Ordering::Relaxed);
            let mut removed = removed_master.write_owned().await;
            self_ref.cache.append(&mut removed);
        }
    }

    /// Clear all temporary files from their specific location, makes use of workers to
    /// quickly clear the files. Does not work recursively.
    async fn clear_temp_files(&mut self) {
        let paths = match std::fs::read_dir(&self.temp_path) {
            Ok(f) => f,
            Err(e) => return error!("unable to read files in cache due to {}", e),
        };
        let mut handles = vec![];
        for entry in paths {
            if let Ok(entry) = entry {
                error!("ENTRY_NAME: {:?}", entry.file_name()); //temp
                match entry.file_name().to_str() {
                    Some(f) if !self.restricted_files.contains(f) => {}
                    Some(_) => continue,
                    None => {
                        warn!("unable to process a file when clearing temporary files");
                        continue;
                    }
                }

                //Skip directories
                match entry.metadata() {
                    Ok(f) if f.is_dir() => {
                        debug!(
                            "encountered directory while clearing temporary files, skipping {:?}",
                            entry.file_name()
                        );
                        continue;
                    }
                    _ => {}
                }

                //Delete this file asynchronously
                handles.push(remove_file(entry.path()));
            }
        }
        join_all(handles).await;
    }

    /// consumes the process into an async runtime which can then be monitored via a file handle
    fn process(mut self) -> std::thread::JoinHandle<()> {
        thread::Builder::new()
            .name(String::from("cache-master"))
            .spawn(move || {
                println!("cache master process started");
                let rt = Runtime::new().unwrap();
                rt.block_on(async move {
                    println!("cache master process runtime started");
                    let mut count = 0;

                    loop {
                        if let Some(msg) = self.rx.recv().await {
                            println!("cache master received message {:?}", msg);
                            match msg {
                                CacheAction::Used(f) => {
                                    self.used(f.0, f.1);
                                    //Only enforce the size of the cache every 100 messages to save compute
                                    match count {
                                        100 => {
                                            self.enforce_cache_size().await;
                                            count = 0;
                                        }
                                        _ => count += 1,
                                    }
                                }
                                CacheAction::Close => {
                                    self.rx.close();
                                }
                            }
                        } else {
                            //Close and drop
                            self.enforce_cache_size().await;
                            self.clear_temp_files().await;
                            break;
                        }
                    }
                });
            })
            .expect("a valid thread handle")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CacheAction {
    Used(([u8; 32], u32)),
    Close,
}

/// A cache driver which is spawned off by the fairing for sending messages back and forth
#[derive(Debug, Clone)]
pub struct Cache {
    sender: mpsc::UnboundedSender<CacheAction>,
}

impl Cache {
    pub fn fairing() -> AdHoc {
        //XXX change for try_on_ignite
        AdHoc::on_ignite("file-cache", move |mut rocket| {
            Box::pin(async move {
                let config = match rocket.state::<Config>() {
                    Some(cfg) => cfg,
                    None => {
                        warn!("config not found while attempting to attach cache, initalising config");
                        rocket = rocket.manage(Config::fairing());
                        rocket.state::<Config>().unwrap()
                    },
                };

                let (cache_manager, sender) = CacheManager::new(
                    PathBuf::from(config.CACHE_PATH()),
                    PathBuf::from(config.TEMP_PATH()),
                    config.MAX_CACHE_SIZE() as u64,
                ).unwrap();

                rocket
                    .manage(Box::new(cache_manager.process()))
                    .manage(Cache { sender })
            })
        })
    }

    /// New Item created/used, and this should be reflected by the cache
    pub async fn used(&self, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes: [u8; 32] = [0; 32];
        hex::decode_to_slice(
            match path.file_stem() {
                Some(s) => s.as_bytes(),
                None => {
                    return Err(Box::new(std::io::Error::new(
                        ErrorKind::Other,
                        "no extension",
                    )))
                }
            },
            &mut bytes,
        )?;

        let size = tokio::fs::metadata(path.as_path()).await?.len() as u32;
        self.sender.send(CacheAction::Used((bytes, size)))?;
        Ok(())
    }

    /// Attempt to close the cache, and destroy it. This will cause all future
    /// messages to the cache to return an error. This should only be called
    /// when closing the entire api.
    #[allow(unused_must_use)]
    pub fn flush_and_destroy(self) {
        self.sender.send(CacheAction::Close);
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Cache {
    type Error = Infallible;
    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let state = req
            .rocket()
            .state::<Cache>()
            .expect("cache manager attached");
        rocket::request::Outcome::Success(state.clone())
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use crate::Cache;
    use config::{Config, PathType};
    use rocket::{get, http::Status, local::blocking::Client, routes, uri};
    use utils::{sha_256_hash};
    use utils::test_utils::AlteredToml;

    #[get("/")]
    async fn hello_world(cache: Cache) -> &'static str {
        cache
            .used(PathBuf::from(format!(
                "{}.wav",
                sha_256_hash("Long input string")
            )))
            .await
            .unwrap();
        "success!"
    }

    /// Test that the caching actually works as intended
    /// the cache should not allow a buildup over a certain size to occur
    #[test]
    fn test_size_limits() {
        // TODO
    }

    /// Test that creating and attaching a fairing does not cause a panic
    #[test]
    fn rocket_fairing() {
        let replace_search = "CACHE_PATH = \"./cache\"";
        let replace_data = "CACHE_PATH = \"../../../cache\"";

        let _t = AlteredToml::new(replace_search, &replace_data, PathType::General, PathBuf::from("../../../config"));

        let cfg: Config = Config::new(PathBuf::from("../../../config")).unwrap();

        std::fs::write(
            format!("{}.wav", sha_256_hash("Long input string")),
            "data, of the long and terrible kind",
        )
        .unwrap();

        let rocket = rocket::build()
            .mount("/", routes![hello_world])
            .manage(cfg)
            .attach(Cache::fairing());

        let client = Client::tracked(rocket).expect("valid rocket instance");
        let response = client.get(uri!(hello_world)).dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().unwrap(), "success!");

        std::fs::remove_file(format!("{}.wav", sha_256_hash("Long input string"))).unwrap();

        std::mem::drop(_t);
    }
}
