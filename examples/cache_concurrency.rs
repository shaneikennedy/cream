use cream::Cache;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    // Create cache with max 10 keys, default 2s TTL, 50ms cleanup interval
    let cache: Arc<Cache<i32, String>> = Arc::new(
        Cache::new()
            .with_max_size(10)
            .with_ttl(Duration::from_secs(2)),
    );

    // Insert initial data
    cache.put(1, "one".to_string());
    cache.put(2, "two".to_string());
    cache.put(3, "three".to_string());

    let mut handles = vec![];

    // Spawn 5 reader threads
    for i in 0..5 {
        let cache = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            if let Some(value) = cache.get(&1) {
                println!("Reader {} got key 1: {}", i, value);
            } else {
                println!("Reader {}: key 1 expired or missing", i);
            }
            let keys: Vec<i32> = cache.keys().collect();
            println!("Reader {} keys: {:?}", i, keys);
            let values: Vec<String> = cache.values().collect();
            println!("Reader {} values: {:?}", i, values);
            assert!(
                cache.exists(&1) || cache.get(&1).is_none(),
                "Consistency check"
            );
            thread::sleep(Duration::from_millis(100));
        }));
    }
    // Wait for cleanup
    thread::sleep(Duration::from_secs(2));

    // Spawn 2 writer threads
    for i in 0..2 {
        let cache = Arc::clone(&cache);
        handles.push(thread::spawn(move || {
            cache.put(10 + i, format!("value{}", 10 + i));
            println!("Writer {} added key {}", i, 10 + i);
            thread::sleep(Duration::from_millis(50));
        }));
    }

    // Wait for threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify final state
    let keys: Vec<i32> = cache.keys().collect();
    println!("Final keys: {:?}", keys);
    assert!(cache.get(&1).is_none(), "Key 1 should be expired");
    assert!(cache.get(&2).is_none(), "Key 2 should be expired");
    assert!(cache.get(&3).is_none(), "Key 3 should be expired");
    assert_eq!(
        cache.get(&10),
        Some("value10".to_string()),
        "Key 10 should exist"
    );
    assert_eq!(
        cache.get(&11),
        Some("value11".to_string()),
        "Key 11 should exist"
    );
}
