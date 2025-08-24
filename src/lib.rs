use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex, RwLock},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// The Cache structure, a generic, thread-safe in memory cache with support for size constraints and time-to-live
pub struct Cache<K, V> {
    data: Arc<RwLock<BTreeMap<K, (V, Instant)>>>,
    max_keys: Mutex<Option<usize>>,
    ttl: Mutex<Option<Duration>>,
    cleanup_thread: Mutex<Option<JoinHandle<()>>>,
    stop: Arc<RwLock<bool>>,
    insert_order: Arc<RwLock<VecDeque<K>>>,
}

impl<K, V> Drop for Cache<K, V> {
    /// We need to send a "stop signal" to the TTL thread
    /// in order to properly cleanup the Cache instance.
    fn drop(&mut self) {
        *self.stop.write().unwrap() = true;
        if let Some(h) = self.cleanup_thread.lock().unwrap().take() {
            let res = h.join();
            match res {
                Ok(_) => (),
                Err(_) => panic!("Problem dropping cache"), // Honestly not sure what to do here
            }
        }
    }
}

impl<K: Ord + Clone + Sync + Send + 'static, V: Clone + Sync + Send + 'static> Default
    for Cache<K, V>
{
    /// A new Cache with the default setting: unbound size and no time-to-live.
    fn default() -> Self {
        Cache::new()
    }
}

impl<K: Ord + Clone + Sync + Send + 'static, V: Clone + Sync + Send + 'static> Cache<K, V> {
    /// A new Cache with the default setting: unbound size and no time-to-live.
    pub fn new() -> Self {
        Cache {
            data: Arc::new(RwLock::new(BTreeMap::new())),
            max_keys: Mutex::new(None),
            ttl: Mutex::new(None),
            cleanup_thread: Mutex::new(None),
            stop: Arc::new(RwLock::new(false)),
            insert_order: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Updates the current cache with a given max_size that
    /// will be considered when inserting new keys.
    /// The cache will evict the "oldest" key in the cache once
    /// it reaches its `max_size`
    pub fn with_max_size(self, size: usize) -> Self {
        *self.max_keys.lock().unwrap() = Some(size);
        self
    }

    /// Updates the current cache with a time-to-live (TTL) for all keys in the cache.
    /// This will start a background thread that purges any keys past their TTL.
    /// Additionally, setting a ttl means that all cache "read" operations (get, exists, key iteration)
    /// will consider the TTL such that the reader will never see values that are expired,
    /// regardless if they have been cleaned up or not.
    pub fn with_ttl(self, ttl: Duration) -> Self {
        self.ttl.lock().unwrap().replace(ttl);
        let stop_flag = self.stop.clone();
        let data = self.data.clone();
        let insert_order = self.insert_order.clone();
        self.cleanup_thread
            .lock()
            .unwrap()
            .replace(thread::spawn(move || {
                while !*stop_flag.read().unwrap() {
                    let mut data_guard = data.write().unwrap();
                    data_guard.retain(|_, (_, inst)| inst.elapsed() < ttl);
                    drop(data_guard);
                    let mut insert_guard = insert_order.write().unwrap();
                    insert_guard.retain(|k| data.read().unwrap().contains_key(k));
                    drop(insert_guard);
                    thread::sleep(Duration::from_millis(50));
                }
            }));
        self
    }

    /// Puts a value into the cache for a given key.
    pub fn put(&self, key: K, val: V) -> Option<V> {
        if let Some(max) = *self.max_keys.lock().unwrap()
            && self.data.read().unwrap().len() >= max
        {
            // Yeet the oldest key
            // In theory i shouldn't need to check this, since if there are
            // any keys in the cache, let alone the max number of keys,
            // there must be a value for the "oldest"
            let oldest = self.insert_order.write().unwrap().pop_front();
            match oldest {
                Some(o) => self.data.write().unwrap().remove(&o),
                None => None,
            };
        }
        let inserted = self
            .data
            .write()
            .unwrap()
            .insert(key.clone(), (val, Instant::now()))
            .map(|(v, _)| v);
        self.insert_order.write().unwrap().push_back(key);
        inserted
    }

    /// Gets the current value in the cache for the given key. Returns None if
    /// the key does not exist or is past its time-to-live, if it has one.
    pub fn get(&self, key: &K) -> Option<V> {
        let c = self.data.read().unwrap();
        if let Some((v, inst)) = c.get(key) {
            if let Some(ttl) = *self.ttl.lock().unwrap() {
                if inst.elapsed() < ttl {
                    Some(v.clone())
                } else {
                    None
                }
            } else {
                Some(v.clone())
            }
        } else {
            None
        }
    }

    /// Return an iterator over all keys in the cache.
    /// This will exclude any keys that are past the time-to-live.
    pub fn keys(&self) -> impl Iterator<Item = K> {
        let ttl = *self.ttl.lock().unwrap();
        self.data
            .read()
            .unwrap()
            .iter()
            .filter_map(|(k, (_, inst))| {
                if let Some(ttl) = ttl {
                    if inst.elapsed() < ttl {
                        Some(k.clone())
                    } else {
                        None
                    }
                } else {
                    Some(k.clone())
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Return an iterator over all values in the cache.
    /// This will exclude any values for which the key is past the time-to-live.
    pub fn values(&self) -> impl Iterator<Item = V> {
        let ttl = *self.ttl.lock().unwrap();
        self.data
            .read()
            .unwrap()
            .iter()
            .filter_map(|(_, (v, inst))| {
                if let Some(ttl) = ttl {
                    if inst.elapsed() < ttl {
                        Some(v.clone())
                    } else {
                        None
                    }
                } else {
                    Some(v.clone())
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Checks for the presence of a key.
    /// This method will return false for any key past its time-to-live.
    pub fn exists(&self, key: &K) -> bool {
        let binding = self.data.read().unwrap();
        let entry = binding.get(key);
        if self.ttl.lock().unwrap().is_some() {
            match entry {
                Some((_, instant)) => instant.elapsed() < self.ttl.lock().unwrap().unwrap(),
                None => false,
            }
        } else {
            entry.is_some()
        }
    }

    /// Remove a key from the cache. Returns Some(value) on a successful removal
    /// and None if the given key does not exist in the cache.
    pub fn remove(&self, key: &K) -> Option<V> {
        let val = self.data.write().unwrap().remove(key).map(|(v, _)| v);
        match val {
            Some(v) => {
                // This key should be here, but it's not a problem to be safe
                let mut insert_guard = self.insert_order.write().unwrap();
                // if let Some(index) = insert_guard.iter().position(|k| k == key) {
                //     self.insert_order.write().unwrap().remove(index);
                // }

                // I do not understand why `remove` causes a deadlock but retain works
                insert_guard.retain(|k| k != key);
                Some(v)
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn test_cache_init() {
        Cache::<String, i32>::new()
            .with_max_size(15)
            .with_ttl(Duration::from_secs(1));
    }

    #[test]
    fn test_cache_max_keys() {
        let cache = Cache::<String, i32>::new().with_max_size(2);
        cache.put("hello1".into(), 5);
        cache.put("hello2".into(), 6);
        cache.put("hello3".into(), 7);

        assert_eq!(cache.keys().collect::<Vec<_>>().len(), 2);
        assert!(!cache.exists(&"hello1".into()));
    }

    #[test]
    fn test_cache_put_and_get() {
        let cache: Cache<String, i32> = Cache::new();
        cache.put("hello1".into(), 5);
        cache.put("hello2".into(), 6);
        cache.put("hello3".into(), 7);

        let expected: i32 = 5;
        assert_eq!(
            cache.get(&"hello1".to_string()).unwrap().to_owned(),
            expected
        );
        assert_eq!(
            cache.get(&"hello2".to_string()).unwrap().to_owned(),
            expected + 1
        );
        assert_eq!(
            cache.get(&"hello3".to_string()).unwrap().to_owned(),
            expected + 2
        );
    }

    #[test]
    fn test_cache_remove() {
        let cache: Cache<String, i32> = Cache::new();
        cache.put("hello1".into(), 5);
        cache.put("hello2".into(), 5);
        cache.remove(&"hello1".into());

        let expected: i32 = 5;
        assert!(cache.get(&"hello1".to_string()).is_none());
        assert_eq!(
            cache.get(&"hello2".to_string()).unwrap().to_owned(),
            expected
        );
    }

    #[test]
    fn test_cache_ttl() {
        let cache: Cache<String, i32> = Cache::new()
            .with_max_size(15)
            .with_ttl(Duration::from_millis(10));
        cache.put("hello1".into(), 5);
        cache.put("hello2".into(), 6);
        cache.put("hello3".into(), 7);
        thread::sleep(Duration::from_millis(10));
        assert!(cache.keys().collect::<Vec<_>>().is_empty())
    }

    #[test]
    fn test_cache_ttl_some_expired() {
        let cache: Cache<String, i32> = Cache::new()
            .with_max_size(15)
            .with_ttl(Duration::from_millis(200));
        cache.put("hello1".into(), 5);
        thread::sleep(Duration::from_millis(200));
        cache.put("hello2".into(), 6);
        cache.put("hello3".into(), 7);
        cache.put("hello4".into(), 5);
        assert!(cache.keys().collect::<Vec<_>>().len() == 3)
    }
}
