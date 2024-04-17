use easy_async::block_on;
use easy_async::utils::PendingN;

#[test]
fn try_block_on() {
    assert_eq!(block_on(PendingN::new(100, false)), ());
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
    assert_eq!(block_on(async_fib(20)), fib(20));
}
