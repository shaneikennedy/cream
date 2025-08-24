use cream::Cache;

fn main() {
    let cache: Cache<i32, i32> = Cache::new().with_max_size(1);
    for i in 1..5 {
        cache.put(i, i);
        println!("Keys in cache: {:#?}", cache.keys().collect::<Vec<_>>());
    }
}
