use easy_async::block_on;
use easy_async::utils::PendingN;

#[test]
fn lock_worker_many_async() {
    use easy_async::executor::async_task_executor::spawn;

    const NUM: usize = 10000;
    const TO: usize = 100;
    let mut tasks = Vec::with_capacity(NUM);

    let start = std::time::Instant::now();
    for _ in 0..NUM {
        tasks.push(spawn(PendingN::new(TO, false)));
    }

    for task in tasks {
        block_on(task);
    }
    println!("{}ms", start.elapsed().as_millis());
}

#[test]
fn no_output_many_async() {
    use easy_async::executor::no_output_executor::complated;
    use easy_async::executor::no_output_executor::spawn;

    const NUM: usize = 10000;
    const TO: usize = 100;
    let mut tasks = Vec::with_capacity(NUM);

    let start = std::time::Instant::now();
    for _ in 0..NUM {
        tasks.push(spawn(PendingN::new(TO, false)));
    }

    for task in tasks {
        block_on(task);
    }
    assert!(complated());
    println!("{}ms", start.elapsed().as_millis());
}
