use rkit::utils::{init_local_pool, paste, InnerLocalPool, LocalPool, LocalPoolObserver};
init_local_pool!(MY_POOL, 32, Vec<u8>, || Vec::with_capacity(100));

fn main() {
    println!("before len: {}", MY_POOL.len());
    {
        let v = MY_POOL.take().unwrap();
        println!("l:{} c:{}", v.len(), v.capacity());
        println!("using len: {}", MY_POOL.len());
    }
    println!("after len: {}", MY_POOL.len());
}
