use cream::Cache;

fn main() {
    let cache: Cache<String, String> = Cache::new();
    cache.put("Hello".into(), "world".into());
    println!("Hello, {}!", cache.get(&"Hello".into()).unwrap());
}
