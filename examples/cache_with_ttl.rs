use std::{thread, time::Duration};

use cream::Cache;

fn main() {
    let cache: Cache<i32, i32> = Cache::new().with_ttl(Duration::from_secs(5));
    for i in 1..5 {
        println!("inserting {i}");
        cache.put(i, i);
        thread::sleep(Duration::from_secs(1));
    }
    println!("Keys fully inserted");

    while cache.keys().count() > 0 {
        println!("keys remaining: {:#?}", cache.keys().collect::<Vec<_>>());
        thread::sleep(Duration::from_millis(1000));
    }
}
