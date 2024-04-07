#[test]
fn smol_spawn() {
    use smol::spawn;
    // use smol::block_on;
    use easy_async::block_on;

    assert_eq!(block_on(spawn(async { "hello" })), "hello");
}

#[test]
fn spawn() {
    use easy_async::block_on;
    use easy_async::executor::executor::spawn;
    // use smol::block_on;

    assert_eq!(block_on(spawn(async { "hello" })), "hello");
}

async fn async_fib(n: usize) -> usize {
    if n < 2 {
        return 1;
    }

    Box::pin(async_fib(n - 1)).await + Box::pin(async_fib(n - 2)).await
}

fn fib(n: usize) -> usize {
    let (mut a, mut b) = (1, 0);
    for _ in 0..n {
        (a, b) = (a + b, a);
    }
    a
}

#[test]
fn block_on_fib() {
    use easy_async::block_on;
    // use smol::block_on;

    assert_eq!(block_on(async_fib(20)), fib(20));
}

#[test]
fn spawn_fib() {
    use easy_async::block_on;
    use easy_async::executor::executor::spawn;
    // use smol::block_on;

    assert_eq!(block_on(spawn(async_fib(20))), fib(20));
}

#[test]
fn smol_spawn_fib() {
    use easy_async::block_on;
    use smol::spawn;
    // use smol::block_on;

    assert_eq!(block_on(spawn(async_fib(20))), fib(20));
}

#[test]
fn many_async() {
    use easy_async::executor::executor::spawn;
    // use smol::spawn;
    use easy_async::block_on;
    // use smol::block_on;

    use easy_async::utils::pending_n::PendingN;

    const NUM: usize = 100;
    const TO: usize = 100;
    let mut tasks = Vec::with_capacity(NUM);

    let start = std::time::Instant::now();
    for _ in 0..NUM {
        tasks.push(spawn(PendingN::new(TO)));
    }

    for task in tasks {
        block_on(task);
    }
    println!("{}ms", start.elapsed().as_millis());
}

#[test]
fn smol_many_async() {
    // use easy_async::executor::executor::spawn;
    use easy_async::block_on;
    use smol::spawn;
    // use smol::block_on;

    use easy_async::utils::pending_n::PendingN;

    const NUM: usize = 100;
    const TO: usize = 100;
    let mut tasks = Vec::with_capacity(NUM);

    let start = std::time::Instant::now();
    for _ in 0..NUM {
        tasks.push(spawn(PendingN::new(TO)));
    }

    for task in tasks {
        block_on(task);
    }
    println!("{}ms", start.elapsed().as_millis());
}
