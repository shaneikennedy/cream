# C.R.E.A.M (Cache rules everything around me)

A generic thread-safe in-memory cache that supports size limits and TTL.

## Features
- Explicit size or unbounded, configurable
- Time-to-live configuration
- Iterating over keys in order, determined by the Ord trait
- Iterating over values in order of the keys, determined by the Ord trait

## Examples

### Basic Cache usage

``` rust
cream on main
❯ cat examples/cache.rs
use cream::Cache;

fn main() {
    let mut cache: Cache<String, String> = Cache::new();
    cache.put("Hello".into(), "world".into());
    println!("Hello, {}!", cache.get(&"Hello".into()).unwrap());
}

cream on main
❯ cargo run --example cache
Hello, world!
```


### Cache with a time-to-live (TTL) for entries
``` rust
cream on main
❯ cat examples/cache_with_ttl.rs
use std::{thread, time::Duration};

use cream::Cache;

fn main() {
    let mut cache: Cache<i32, i32> = Cache::new().with_ttl(Duration::from_secs(5));
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

cream on main
❯ cargo run --example cache_with_ttl
inserting 1
inserting 2
inserting 3
inserting 4
Keys fully inserted
keys remaining: [1, 2, 3, 4]
keys remaining: [2, 3, 4]
keys remaining: [3, 4]
keys remaining: [4]
```

### Cache with max_size

``` rust
cream on main
❯ cat examples/cache_with_size.rs
use cream::Cache;

fn main() {
    let mut cache: Cache<i32, i32> = Cache::new().with_max_size(1);
    for i in 1..5 {
        cache.put(i, i);
        println!("Keys in cache: {:#?}", cache.keys().collect::<Vec<_>>());
    }
}

cream on main
❯ cargo run --example cache_with_size
Keys in cache: [1]
Keys in cache: [2]
Keys in cache: [3]
Keys in cache: [4]
```
