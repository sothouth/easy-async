use async_std::task::spawn_blocking;
use easy_async::block_on;
use easy_async::spawn;

async fn fib(n: usize) -> usize {
    if n < 2 {
        1
    } else {
        spawn(async move { fib(n - 1).await }).await + spawn(async move { fib(n - 2).await }).await
    }
}

fn main() {
    block_on(async {
        for _ in 0..10000 {
            spawn!(spawn_blocking(|| {})).await;
        }
    });
}
