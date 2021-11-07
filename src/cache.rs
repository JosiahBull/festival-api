//! This module handles caching for the api.
//! A configurable number of words, or a max total size of files should be kept.
//! The data flow is as follows
//! req -> check if item in cache
//!     -> item in cache, increment popularity score, return item
//!     -> not in cache, generate item fresh, update popularity score
//!         -> if space in cache, save item.
//!         -> if no space in cache, check if our score is greater (10pts?) than the lowest popularity score
//!             -> if greater, bump that item, update the lowest popularity score number, cache this item
//!             -> else, do nothing.
//!
//! The API for this cache is designed to be flexible and reusable, so it makes extensive use of generics.
//! This means that to use it expect to have to implement functions to pass into the api.

use priority_queue::DoublePriorityQueue;
use std::{collections::HashMap, hash::Hash, io::ErrorKind, marker::PhantomData};

/// The number of bytes in a mb.
const BYTES_IN_MB: usize = 1_000_000;

/// A type indicating that this return type may be cached properly in the api.
/// These methods are required to save and load from the disk.
#[rocket::async_trait]
trait Cachable<U> {
    /// Load the underlying file from the disk, or generate it fresh if it's not there.
    async fn load_underlying(&self) -> Result<U, std::io::Error>;

    /// Get the total size this object is taking up on the disk. Should fail if the file has not been
    /// saved to the disk.
    async fn size_on_disk(&self) -> Result<usize, std::io::Error>;

    /// Save this file to the disk, so that it is cached for future use. May require regeneration of the 
    /// file.
    async fn save_on_disk(&self) -> Result<(), std::io::Error>;

    /// Remove the underlying file from the disk.
    async fn remove_from_disk(&self) -> Result<(), std::io::Error>;
}

/// A struct which wraps cacheable data for the cache. This is useful as we can store information about the 
/// stored item, which can be used when making decisions about whether to cache or not.
#[derive(Debug, Clone)]
struct Info<G, U>
where
    G: Cachable<U> + Send + Sync,
{
    /// Number of times this data has been requested
    uses: usize,
    /// Whether it is currently cached to disk
    cached: bool,
    /// The wrapped internal data
    wrapped: G,
    #[doc(hidden)]
    _return_type: PhantomData<U>,
}

/// A cache for storing frequently used data. Will automatically attempt to cache popular items over time, 
/// decaching less popular items as required.
struct Cache<T, G, U>
where
    T: Hash + Eq + Clone,
    G: Cachable<U> + Send + Sync,
{
    /// Maximum number of items to cache
    max_to_cache: usize,
    /// Maximum size of items to cache
    max_size_of_cache_bytes: usize,

    /// The current count of items cached
    count: usize,
    /// THe current size of all cached items on the disk =
    size_on_disk: usize,

    /// When an item gets replaced, how large should the "bump" be to prevent
    /// it from being quickly deseated. This is very important as it stops frequent swapping of lower
    /// cached items consuming system resources.
    uses_threshold: usize,

    /// A double priority queue which stores the itemes in least and most popular form.
    /// This is backed by a HashMap, which means we get O(log(n)) for most operations in the worst 
    /// case.
    priority: DoublePriorityQueue<T, usize>,

    /// The cache itself, stores data about a variety of objects. Note that we wrap objects with
    /// an info struct to track information about individual objects.
    cache: HashMap<T, Info<G, U>>,
}

impl<T, G, U> Cache<T, G, U>
where
    T: Hash + Eq + Clone,
    G: Cachable<U> + std::cmp::PartialEq + Send + Sync,
{
    /// Create a new cache, with a specific max number of items and max size on disk.
    fn new(max_items: usize, max_size: usize) -> Cache<T, G, U> {
        Cache::<T, G, U> {
            max_to_cache: max_items,
            max_size_of_cache_bytes: max_size,
            ..Default::default()
        }
    }

    /// Change the uses_threshold value
    fn set_threshold(&mut self, threshold: usize) {
        self.uses_threshold = threshold;
    }

    /// Get the raw item, without running the function which actually gets the data it contains internally.
    fn get_raw(&self, key: &T) -> Option<&G> {
        if let Some(item) = self.cache.get(key) {
            Some(&item.wrapped)
        } else {
            None
        }
    }

    /// Get information about an item
    fn get_info(&self, key: &T) -> Option<&Info<G, U>> {
        if let Some(item) = self.cache.get(key) {
            Some(item)
        } else {
            None
        }
    }

    /// Check whether the provided item can be cached, and caches it if so.
    /// If the item was cached returns true, if not returns false.
    async fn check_popularity(&mut self, key: &T) -> Result<bool, std::io::Error> {
        let space_available = self.space_available();
        let item = self.cache.get_mut(key).unwrap();
        if item.cached {
            //Already cached!
            return Ok(true);
        }

        //If there is space in the cache, always cache.
        if space_available {
            item.cached = true;
            item.wrapped.save_on_disk().await?;
            self.size_on_disk += item.wrapped.size_on_disk().await?;
            self.count += 1;
            self.priority.push(key.clone(), item.uses);
            Ok(true)
        } else {
            //Check if this item has enough popularity to be cached!
            //TODO fix bug here that could cause size to get away from us
            let min = self.priority.peek_min();
            if let Some(min) = min {
                if item.uses > *min.1 {
                    //Cache new item
                    item.cached = true;
                    item.wrapped.save_on_disk().await?;
                    self.size_on_disk += item.wrapped.size_on_disk().await?;
                    self.priority
                        .push(key.clone(), item.uses + self.uses_threshold);

                    //Decache existing lowest item
                    let decache_key = self.priority.pop_min().unwrap().0;
                    let decache_item = self.cache.get_mut(&decache_key).unwrap();
                    self.size_on_disk -= decache_item.wrapped.size_on_disk().await?;
                    decache_item.wrapped.remove_from_disk().await?;
                    decache_item.cached = false;
                    Ok(true)
                } else {
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        }
    }

    /// Loads the data from an item, and returns it to the user.
    /// Requires a mutable reference as it updates the popularity of this item internally.
    async fn load(&mut self, key: T) -> Result<Option<U>, std::io::Error> {
        if self.cache.contains_key(&key) {
            let item = self.cache.get_mut(&key).unwrap();
            item.uses += 1;
            let was_cached = item.cached;

            if self.check_popularity(&key).await? && was_cached {
                //Item already cached, lets just increase it's popularity
                //TODO Small bug here where a newly cached item will get an extra "use"... it's fairly harmless though so is it worth fixing?
                self.priority.change_priority_by(&key, |x| {
                    *x += 1;
                });
            }

            Ok(Some(
                self.cache
                    .get(&key)
                    .unwrap()
                    .wrapped
                    .load_underlying()
                    .await?,
            ))
        } else {
            Ok(None)
        }
    }

    /// Add a new entry to the cache, which can be retrieved at a later date.
    /// Will automatically save this to the disk if space is available.
    async fn insert(&mut self, key: T, value: G) -> Result<(), std::io::Error> {
        // Check if item already in the cache
        // If it's there, update it
        // If not insert it
        if !self.contains_item(&key) {
            //Not in cache, lets add it.
            self.cache.insert(
                key.clone(),
                Info {
                    wrapped: value,
                    uses: 0,
                    cached: false,
                    _return_type: PhantomData,
                },
            );

            //Check if we should cache this item!
            self.check_popularity(&key).await?;

            Ok(())
        } else {
            Err(std::io::Error::new(
                ErrorKind::AlreadyExists,
                "Key already exists in cache",
            ))
        }
    }

    /// Collect a reference to the underlying backing HashMap. This is primarily useful for testing,
    /// but can also be useful if you wish to manually retrieve data from a stored object.
    fn get_underlying(&self) -> &HashMap<T, Info<G, U>> {
        &self.cache
    }

    /// An unsafe function, allows manual editing of the underlying backing hashmap. Editing this 
    /// directly may break the entire cache, as there are many values which must remain perfectly in 
    /// line for this to succeed.
    unsafe fn get_underlying_mut(&mut self) -> &mut HashMap<T, Info<G, U>> {
        &mut self.cache
    }

    /// Check whether the cache contains a given key
    fn contains_item(&self, key: &T) -> bool {
        self.cache.contains_key(key)
    }

    /// Check if there is space available in the cache
    fn space_available(&self) -> bool {
        self.count < self.max_to_cache && self.size_on_disk < self.max_size_of_cache_bytes
    }
}

impl<'a, T, G, U> Default for Cache<T, G, U>
where
    T: Hash + Eq + Clone,
    G: Cachable<U> + Send + Sync,
{
    fn default() -> Cache<T, G, U> {
        Cache {
            max_to_cache: 100,
            max_size_of_cache_bytes: 10 * BYTES_IN_MB,
            count: 0,
            size_on_disk: 0,
            uses_threshold: 5,
            cache: HashMap::default(),
            priority: DoublePriorityQueue::new(),
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod test {
    use priority_queue::DoublePriorityQueue;

    use super::{Cachable, Cache, Info};
    use crate::rocket::tokio;
    use std::{collections::HashMap, marker::PhantomData};

    #[derive(Debug, PartialEq, Clone)]
    struct Item {
        data: i64,
    }

    #[rocket::async_trait]
    impl Cachable<i64> for Item {
        async fn load_underlying(&self) -> Result<i64, std::io::Error> {
            Ok(self.data)
        }

        async fn size_on_disk(&self) -> Result<usize, std::io::Error> {
            Ok(1)
        }

        async fn save_on_disk(&self) -> Result<(), std::io::Error> {
            Ok(()) //For the purposes of testing, we aren't going to save to the disk
        }

        async fn remove_from_disk(&self) -> Result<(), std::io::Error> {
            Ok(()) //For the purposes of testing, we aren't going to save to the disk
        }
    }

    /// Test that an initally inserted item which has been cached can be decached by a newer more popular item.
    #[rocket::tokio::test]
    async fn popularity_caching() {
        let mut cache: Cache<String, Item, i64> = Cache::new(1, 1000);

        cache.set_threshold(2); //Needs at least 2 uses more than the minimum item to become most popular!

        cache
            .insert("Item1".to_string(), Item { data: 5 })
            .await
            .unwrap();

        //Assert that the item has been cached
        assert!(cache.get_info(&String::from("Item1")).unwrap().cached);

        //Use the item 5 times, so it has 5 "uses"
        for i in 0..5 {
            let item = cache.load(String::from("Item1")).await.unwrap().unwrap();
            assert_eq!(item, 5);
            assert!(cache.get_info(&String::from("Item1")).unwrap().cached); //Still cached!
            assert_eq!(cache.get_info(&String::from("Item1")).unwrap().uses, i + 1);
            //Check uses match
        }

        //Assert that the minimum number required is now 5
        assert_eq!(*cache.priority.peek_min().unwrap().1, 5);

        //Create a new item, use it enough to become cached
        cache
            .insert("Item2".to_string(), Item { data: 3 })
            .await
            .unwrap();
        for i in 0..5 {
            let item = cache.load(String::from("Item2")).await.unwrap().unwrap();
            assert_eq!(item, 3);
            assert!(!cache.get_info(&String::from("Item2")).unwrap().cached); //Not cached!
            assert_eq!(cache.get_info(&String::from("Item2")).unwrap().uses, i + 1);
        }

        //Using 1 more time should make this item become cached, and decache the other item
        let item = cache.load(String::from("Item2")).await.unwrap().unwrap();
        assert_eq!(item, 3);
        assert!(cache.get_info(&String::from("Item2")).unwrap().cached); //Cached!

        //Check the other item has been decached
        let item = cache.load(String::from("Item1")).await.unwrap().unwrap();
        assert_eq!(item, 5);
        assert!(!cache.get_info(&String::from("Item1")).unwrap().cached); //Not cached!

        //Check that the minimum required uses has increased
        let min_cached_val = cache.priority.peek_min().unwrap().1;
        let expected_min_cached_val =
            cache.get_info(&String::from("Item2")).unwrap().uses + cache.uses_threshold;
        assert_eq!(*min_cached_val, expected_min_cached_val);
    }

    /// Ensure cache size limits work correctly, both for number and size of files.
    /// Will create a cache that can store 5 files, of up to 100 bytes total.
    /// Files are cleaned up at the conclusion (or failure) of the test.
    #[rocket::tokio::test]
    async fn size_limits_num_items() {
        let mut cache: Cache<String, Item, i64> = Cache::new(5, 100);

        //We should be able to insert 100 items, but only the first 5 should be cached.
        //Cache 100 items
        for i in 0..100 {
            let item = Item { data: i };
            cache.insert(format!("Number{}", i), item).await.unwrap();
        }

        assert!(!cache.space_available());

        //Check how many are cached
        let mut total = 0;
        for i in cache.get_underlying().clone().into_values() {
            if i.cached {
                total += 1;
            }
        }
        assert_eq!(total, 5);
    }

    /// Validate that we will not store too great a size of files.
    #[rocket::tokio::test]
    async fn test_size_limits_total_size() {
        let mut cache: Cache<String, Item, i64> = Cache::new(1000, 5);

        //We should be able to insert 100 items, but only the first 5 should be cached.
        //Cache 100 items
        for i in 0..100 {
            let item = Item { data: i };
            cache.insert(format!("Number{}", i), item).await.unwrap();
        }

        assert!(!cache.space_available());

        //Check how many are cached
        let mut total = 0;
        for i in cache.get_underlying().clone().into_values() {
            if i.cached {
                total += 1;
            }
        }
        assert_eq!(total, 5);
    }

    /// Ensures the basic functionality of the caching system.
    /// Inserting items, checking that those items exist, and so on.
    #[rocket::tokio::test]
    async fn basic_functionality() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        //Check that we can insert items
        cache
            .insert(String::from("Item1"), Item { data: 6 })
            .await
            .unwrap();

        //Validate inserted item
        let d = cache.get_underlying();
        assert_eq!(d.get("Item1").expect("an item").uses, 0);
        assert_eq!(d.get("Item1").expect("an item").cached, true);
        assert_eq!(d.get("Item1").expect("an item")._return_type, PhantomData);

        //Check for an item that exists and one that doesn't exist
        let exists: bool = cache.contains_item(&String::from("Item1"));
        assert!(exists);

        let exists: bool = cache.contains_item(&String::from("Item2"));
        assert!(!exists);

        //Attempt to collect a (raw) item and the information on it
        let item: Option<&Item> = cache.get_raw(&String::from("Item1"));
        assert_eq!(item.unwrap().data.to_owned(), 6);

        //Attempt to collect a (raw) item that doesn't exist
        let item: Option<&Item> = cache.get_raw(&String::from("Item5"));
        assert_eq!(item, None);

        //Collect information on an item that exists
        let item: Option<&Info<Item, i64>> = cache.get_info(&String::from("Item1"));
        assert!(item.is_some());
        let info = item.unwrap();
        assert_eq!(info.uses, 0);
        assert_eq!(info.cached, true);
        assert_eq!(info.wrapped, Item { data: 6 });

        //Collect information on an item that doesn't exist
        let item: Option<&Info<Item, i64>> = cache.get_info(&String::from("Item5"));
        assert!(item.is_none());

        //Attempt to load an items ""true"" data that we want.
        let true_data: Option<i64> = cache.load(String::from("Item1")).await.unwrap();
        assert_eq!(true_data.unwrap(), 6);

        //Attempt to load an item which doesn't exist
        let doesnt_exist = cache.load(String::from("Item5")).await.unwrap();
        assert!(doesnt_exist.is_none());
    }

    /// Ensure that inserting a duplicate key into the cache causes a failure.
    #[rocket::tokio::test]
    async fn duplicate_key() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        //Should be fine
        cache
            .insert(String::from("Item2"), Item { data: 8 })
            .await
            .unwrap();

        //Check that inserting a duplicate item works as intended
        let err = cache
            .insert(String::from("Item2"), Item { data: 9 })
            .await
            .unwrap_err();

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(err.to_string(), "Key already exists in cache");

        assert_eq!(cache.load(String::from("Item2")).await.unwrap().unwrap(), 8);
    }

    /// Test getting the backing HashMap.
    #[rocket::tokio::test]
    async fn underlying() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        cache
            .insert(String::from("Item2"), Item { data: 92 })
            .await
            .unwrap();

        let underlying: &HashMap<String, Info<Item, i64>> = cache.get_underlying();

        let keys = underlying.clone().into_keys();
        let vals = underlying.clone().into_values();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys.last().unwrap(), String::from("Item2"));
        assert_eq!(vals.last().unwrap().wrapped.data, 92);
    }

    /// Test that default spawns with the expected values
    #[test]
    fn test_default() {
        let cache: Cache<String, Item, i64> = Cache::default();
        assert_eq!(cache.max_to_cache, 100);
        assert_eq!(cache.max_size_of_cache_bytes, 10_000_000);
        assert_eq!(cache.count, 0);
        assert_eq!(cache.uses_threshold, 5);
        assert!(cache.cache.len() == 0);
        assert_eq!(cache.priority, DoublePriorityQueue::new());
    }

    /// Test that the default insertion is good
    #[rocket::tokio::test]
    async fn test_inserted_defaults() {
        let mut cache: Cache<String, Item, i64> = Cache::new(0, 0);

        cache
            .insert(String::from("Item2"), Item { data: 92 })
            .await
            .unwrap();

        let underlying = cache.get_underlying();
        assert_eq!(underlying.get("Item2").expect("a valid item").cached, false);
        assert_eq!(underlying.get("Item2").expect("a valid item").uses, 0);
        assert_eq!(
            underlying.get("Item2").expect("a valid item")._return_type,
            PhantomData
        );
    }
}

///// Test Implementation of the cache as a fairing /////
// struct TestCache {
// data: usize,
// db: Option<DbConn>,
// }

// impl TestCache {
// async fn make_request(&self) -> Option<models::User> {
//     if let Some(f) = &self.db {
//         return common::find_user_in_db(f, common::SearchItem::Id(1)).await.unwrap()
//     }
//     None
// }
// }

// #[rocket::async_trait]
// impl rocket::fairing::Fairing for TestCache {
// fn info(&self) -> rocket::fairing::Info {
//     rocket::fairing::Info {
//         name: "Test Cache Implementation",
//         kind: Kind::Ignite
//     }
// }

// async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
//     //Get a db instance
//     let db = DbConn::get_one(&rocket).await.unwrap();

//     //Initialize our test friend
//     let cache = TestCache {
//         data: 5,
//         db: Some(db)
//     };

//     //Save him to a local state
//     let new_rocket = rocket.manage(cache);

//     //Return our succesfully attached fairing!
//     rocket::fairing::Result::Ok(new_rocket)
// }
// }

// impl Default for TestCache {
// fn default() -> Self {
//     TestCache {
//         data: 5,
//         db: None,
//     }
// }
// }

/////fairing cache implemntation tests/////
// .attach(testcache)
// .manage(friend)
// .attach(rocket::fairing::AdHoc::on_liftoff("Freds", |rocket| {
//     Box::pin(async move {
//         friend.fetch_update(std::sync::atomic::Ordering::Relaxed, std::sync::atomic::Ordering::Relaxed, |_| Some(4));
//     })
// }))
