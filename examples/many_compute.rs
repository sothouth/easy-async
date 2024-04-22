use async_std::task::spawn_blocking;
use easy_async::block_on;

fn main() {
    const N: usize = 1000;
    const M: usize = 30;

    let mut handles = Vec::with_capacity(N);

    for _ in 0..N {
        handles.push(spawn_blocking(|| fib(M)));
    }

    for handle in handles {
        println!("{}", block_on(handle));
    }
}

fn fib(n: usize) -> usize {
    if n < 2 {
        return 1;
    }

    fib(n - 1) + fib(n - 2)
}
