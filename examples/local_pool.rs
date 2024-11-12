use rkit::utils::local_pool::*;
init_local_pool!(MY_POOL, 32, Vec<u8>, || Vec::with_capacity(100));

fn main() {
    println!("Pool len before use: {}", MY_POOL.len());
    {
        let v = MY_POOL.take().unwrap();
        println!("vector len:{} capacity:{}", v.len(), v.capacity());
        println!("Pool len taking a vec: {}", MY_POOL.len());
    }
    println!("Pool len after drop the vec: {}", MY_POOL.len());
}
