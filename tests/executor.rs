use easy_async::block_on;
use easy_async::utils::PendingN;

#[test]
fn smol_many_async() {
    use smol::block_on;
    use smol::spawn;

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
    println!("smol: {}ms", start.elapsed().as_millis());
}

#[test]
fn easy_async_many_async() {
    use easy_async::executor::task_count;
    use easy_async::spawn;

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
    assert!(task_count() == 0);
    println!("easy-async: {}ms", start.elapsed().as_millis());
}
