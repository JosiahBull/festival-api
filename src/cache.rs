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

// General TODOs
// TODO Improve Error handling with a custom error type that implements Into<Response>.

use std::{collections::HashMap, hash::Hash, io::ErrorKind, marker::PhantomData};

const BYTES_IN_MB: usize = 1048576;
//TODO update error types to something more usuable?
#[rocket::async_trait]
trait Cachable<U> {
    async fn load_underlying(&self) -> Result<U, std::io::Error>;
    async fn size_on_disk(&self) -> Result<usize, std::io::Error>;
    async fn save_on_disk(&self) -> Result<(), std::io::Error>;
    async fn remove_from_disk(&self) -> Result<(), std::io::Error>;
}

#[derive(Debug, Clone)]
struct Info<G, U>
where
    G: Cachable<U> + Send + Sync,
{
    uses: usize,
    cached: bool,
    wrapped: G,
    _return_type: PhantomData<U>,
}

struct Cache<T, G, U>
where
    T: Hash,
    G: Cachable<U> + Send + Sync,
{
    max_to_cache: usize,
    max_size_of_cache_bytes: usize,

    count: usize,
    size_on_disk: usize,

    min_uses: usize,
    uses_threshold: usize,

    cache: HashMap<T, Info<G, U>>,
}

impl<T, G, U> Cache<T, G, U>
where
    T: Hash + Eq,
    G: Cachable<U> + std::cmp::PartialEq + Send + Sync,
{
    fn new(max_items: usize, max_size: usize) -> Cache<T, G, U> {
        let mut def = Self::default();
        def.max_to_cache = max_items;
        def.max_size_of_cache_bytes = max_size;
        def
    }

    /// Get the raw item, without running the function which actually gets the data it contains internally.
    fn get_raw(&self, key: &T) -> Option<&G> {
        if let Some(i) = self.cache.get(&key) {
            Some(&i.wrapped)
        } else {
            None
        }
    }

    /// Loads the data from an item, and returns it to the user.
    /// Requires a mutable reference as it updates the popularity of this item
    /// internally.
    async fn load(&mut self, key: T) -> Result<Option<U>, std::io::Error> {
        if let Some(item) = self.cache.get_mut(&key) {
            item.uses += 1;
            //TODO check if this item should be cached here, push into function

            let data = item.wrapped.load_underlying().await?;

            Ok(Some(data))
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
        if self.contains_item(&key) {
            //This item has been seen before
            //Lets update it's uses and popularity
            let item: &mut Info<G, U> = self.cache.get_mut(&key).unwrap();

            if value != item.wrapped {
                return Err(std::io::Error::new(ErrorKind::AlreadyExists, "Unable to insert due to existing item with this key!"))
            }

            item.uses += 1;
            let uses = item.uses;

            if uses > self.min_uses {
                self.min_uses = uses + self.uses_threshold;

                //Decache the current item with the minimum uses
                //Note that this could be improved to just statically store an array of items to be decached
                //Rather than needing to always locate the lowest item. As this is a very poorly-optimised process
                //TODO: Optimise + test the crap out of this
                let mut item: Option<&mut Info<G, U>> = None;
                let mut lowest_uses: usize = 0;
                for i in self.cache.values_mut() {
                    if i.uses <= lowest_uses {
                        lowest_uses = i.uses;
                        item = Some(i);
                    }
                }
                if item.is_none() {
                    panic!("Caching of item failed du()e to unknown error!");
                }
                let item = item.unwrap();

                //Decache the previous item
                item.cached = false;
                self.size_on_disk -= item.wrapped.size_on_disk().await?;
                item.wrapped.remove_from_disk().await?;

                //Cache our new item as it's more popular
                let item: &mut Info<G, U> = self.cache.get_mut(&key).unwrap();
                item.cached = true;
                item.wrapped.save_on_disk().await?;
                self.size_on_disk += item.wrapped.size_on_disk().await?;
            }
        } else {
            let mut cached = false;

            //Save the data if we can!
            if self.space_available() {
                cached = true;
                value.save_on_disk().await?;
                self.count += 1;
                self.size_on_disk += value.size_on_disk().await?;
            }

            //Wrap the data
            let data: Info<G, U> = Info {
                uses: 1,
                cached,
                wrapped: value,
                _return_type: PhantomData,
            };

            self.cache.insert(key, data);
        }
        Ok(())
    }

    fn get_underlying<'a>(&'a mut self) -> &'a mut HashMap<T, Info<G, U>> {
        &mut self.cache
    }

    fn contains_item(&self, key: &T) -> bool {
        self.cache.contains_key(key)
    }

    fn size(&self) -> usize {
        self.max_to_cache
    }

    /// Check if there is space available in the cache
    fn space_available(&self) -> bool {
        self.count < self.max_to_cache && self.size_on_disk < self.max_size_of_cache_bytes
    }
}

impl<T, G, U> Default for Cache<T, G, U>
where
    T: Hash,
    G: Cachable<U> + Send + Sync,
{
    fn default() -> Cache<T, G, U> {
        Cache {
            max_to_cache: 100,
            max_size_of_cache_bytes: 10 * BYTES_IN_MB,
            count: 0,
            size_on_disk: 0,
            min_uses: 0,
            uses_threshold: 5,
            cache: HashMap::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cachable, Cache, Info};
    use crate::rocket::tokio;
    use std::{collections::HashMap};

    #[derive(Debug, PartialEq, Clone)]
    struct Item {
        data: String,
    }

    #[rocket::async_trait]
    impl Cachable<i64> for Item {
        async fn load_underlying(&self) -> Result<i64, std::io::Error> {
            Ok(42)
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

    /// Ensure cache size limits work correctly, both for number and size of files.
    /// Will create a cache that can store 5 files, of up to 100 bytes total.
    /// Files are cleaned up at the conclusion (or failure) of the test.
    #[rocket::tokio::test]
    async fn test_size_limits_num_items() {
        let mut cache: Cache<String, Item, i64> = Cache::new(5, 100);

        //We should be able to insert 100 items, but only the first 5 should be cached.
        //Cache 100 items
        for i in 0..100 {
            let item = Item { data: format!("Number {} is alive!", i) };
            cache.insert(format!("Number{}", i), item).await.unwrap();
        }

        assert!(!cache.space_available());

        //Check how many are cached
        let mut total = 0;
        for i in cache.get_underlying().clone().into_values() {
            if i.cached {
                total += 1;
            }
        };
        assert_eq!(total, 5);
    }

    /// Validate that we will not store too great a size of files.
    #[rocket::tokio::test]
    async fn test_size_limits_total_size() {
        let mut cache: Cache<String, Item, i64> = Cache::new(1000, 5);

        //We should be able to insert 100 items, but only the first 5 should be cached.
        //Cache 100 items
        for i in 0..100 {
            let item = Item { data: format!("Number {} is alive!", i) };
            cache.insert(format!("Number{}", i), item).await.unwrap();
        }

        assert!(!cache.space_available());

        //Check how many are cached
        let mut total = 0;
        for i in cache.get_underlying().clone().into_values() {
            if i.cached {
                total += 1;
            }
        };
        assert_eq!(total, 5);
    }

    /// Ensures the basic functionality of the caching system.
    /// Inserting items, checking that those items exist, and so on.
    #[rocket::tokio::test]
    async fn basic_functionality() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        assert_eq!(cache.size(), 100);

        //Check that we can insert items
        cache
            .insert(
                String::from("Item1"),
                Item {
                    data: String::from("Hello, world!"),
                },
            )
            .await
            .unwrap();

        //Check for an item that exists and one that doesn't exist
        let exists: bool = cache.contains_item(&String::from("Item1"));
        assert!(exists);

        let exists: bool = cache.contains_item(&String::from("Item2"));
        assert!(!exists);

        //Attempt to collect a (raw) item and the information on it
        let item: Option<&Item> = cache.get_raw(&String::from("Item1"));
        assert_eq!(item.unwrap().data.to_owned(), String::from("Hello, world!"));

        //Attempt to load an items ""true"" data that we want.
        let true_data: Option<i64> = cache.load(String::from("Item1")).await.unwrap();
        assert_eq!(true_data.unwrap(), 42);
    }

    /// Ensure that inserting a duplicate key into the cache causes a failure.
    #[rocket::tokio::test]
    async fn test_duplicate_key() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        //Should be fine
        cache.insert(String::from("Item2"), Item { data: String::from("Things!") }).await.unwrap();

        //Check that inserting a duplicate item works as intended
        cache.insert(String::from("Item2"), Item { data: String::from("Things2!") }).await.unwrap_err();
        
        assert_eq!(cache.get_raw(&String::from("Item2")).unwrap(), &Item { data: String::from("Things!") });
    }

    /// Test getting the backing HashMap. 
    #[rocket::tokio::test]
    async fn test_underlying() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        cache.insert(String::from("Item2"), Item { data: String::from("Things!") }).await.unwrap();

        let underlying: &mut HashMap<String, Info<Item, i64>> = cache.get_underlying();

        let keys = underlying.clone().into_keys();
        let vals = underlying.clone().into_values();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys.last().unwrap(), String::from("Item2"));
        assert_eq!(vals.last().unwrap().wrapped, Item { data: String::from("Things!") });
    }
}
