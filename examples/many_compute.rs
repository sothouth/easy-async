fn fib(n: usize) -> usize {
    match n {
        0 | 1 => 1,
        _ => fib(n - 1) + fib(n - 2),
    }
}

fn fast_fib(n: usize) -> usize {
    let (mut a, mut b) = (1, 0);
    for _ in 0..n {
        (a, b) = (a + b, a);
    }
    a
}

fn main() {
    const N: usize = 1000;
    const M: usize = 30;

    // Compute the `M`th Fibonacci number using an efficient iterative method.
    let res = fast_fib(M);

    // Prepare to store handles for spawned blocking tasks.
    let mut handles = Vec::with_capacity(N);

    // Spawn `N` blocking tasks to compute the `M`th Fibonacci number concurrently.
    for _ in 0..N {
        handles.push(easy_async::spawn_blocking(|| fib(M)));
    }

    // Ensure that each blocking task returns the correct Fibonacci number.
    for handle in handles {
        assert_eq!(easy_async::block_on(handle), res);
    }
}
