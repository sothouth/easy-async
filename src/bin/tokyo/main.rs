use tokio;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
}


#[cfg(test)]
mod tests{
    #[test]
    fn try_cpu(){
        use num_cpus::*;
        println!("cpus: {}", get());
        println!("cpus: {}", get_physical());
    }
}