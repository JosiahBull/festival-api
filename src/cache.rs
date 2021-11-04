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

use std::{collections::HashMap, hash::Hash, marker::PhantomData};

const BYTES_IN_MB: usize = 1048576;

//TODO update error types to something more usuable?
#[rocket::async_trait]
trait Cachable<U> 
{
    async fn load_underlying(&self) -> Result<U, std::io::Error>;
    async fn size_on_disk(&self) -> Result<usize, std::io::Error>;
    async fn save_on_disk(&self) -> Result<(), std::io::Error>;
    async fn remove_from_disk(&self) -> Result<(), std::io::Error>;
}

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
    G: Cachable<U> + Send + Sync,
{
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
            item.uses += 1;        
            let uses = item.uses;

            if uses > self.min_uses {
                self.min_uses = uses + self.uses_threshold;

                //Decache the current item with the minimum uses
                //Note that this could be improved to just statically store this
                //Rather than needing to always relocate the lowest item.
                //As this is a very poorly-optimised process
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
                    panic!("Caching of item failed due to unknown error!");
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
        self.max_to_cache < self.count && self.size_on_disk < self.max_size_of_cache_bytes
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
    use std::collections::HashMap;
    use super::{Cachable, Cache};
    use crate::rocket::tokio;

    #[derive(Debug, PartialEq)]
    struct Item { data: String }

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

    #[rocket::tokio::test]
    async fn basic_functionality() {
        let mut cache: Cache<String, Item, i64> = Cache::default();

        assert_eq!(cache.size(), 100);

        //Check that we can insert items
        cache.insert(String::from("Item1"), Item { data: String::from("Hello, world!") }).await.unwrap();

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

    #[test]
    fn test_duplicate_key() {
        // let mut cache: Cache<String, Item, i64> = Cache::default();

        // let result: Option<Item> = cache.insert(String::from("Item2"), Item { data: String::from("Things!") });
        // assert!(result.is_none());

        // //Check that inserting a duplicate item works as intended
        // let result: Option<Item> = cache.insert(String::from("Item2"), Item { data: String::from("Things!") });
        // assert!(result.is_some());
        // assert_eq!(result.unwrap(), Item { data: String::from("Things!") });
    }

    #[test]
    fn test_underlying() {
        // let mut cache: Cache<String, Item, i64> = Cache::default();

        // let result: Option<Item> = cache.insert(String::from("Item2"), Item { data: String::from("Things!") });
        // assert!(result.is_none());

        // let underlying: &mut HashMap<String, Item> = cache.get_underlying();

        // let keys = underlying.into_keys();
        // let vals = underlying.into_values();

        // assert_eq!(underlying.into_keys().len(), 1);
        // assert_eq!(underlying.into_keys().nth(0).unwrap(), String::from("Item2"));
        // assert_eq!(underlying.into_values().nth(0).unwrap(), Item { data: String::from("Things!") });
    }
}