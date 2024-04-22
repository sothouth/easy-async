# easy-async

`easy-async` provides a simple and easy-to-use asynchronous runtime for Rust. It's designed to make asynchronous programming smooth and straightforward, without the need for complex setups.

This runtime revolves around two core components:

## Asynchronous Executor

The provided executor allows for fast and effortless execution of asynchronous tasks. There's no need to configure or construct a runtime using a Builder. Simply use `spawn` to run your async tasks.

- Specialised `Task` objects designed for direct scheduling into local queues, bypassing the additional need for global queue scheduling, reducing cache misses and the performance overhead of repeated enqueuing and dequeuing.
- A simplified worker block-wake mechanism implemented through `Mutex` for efficient task execution.
- A fixed number of workers launched at startup to improve performance.
- Use of `RESERVE_SIZE` to reserve capacity for future task self-scheduling, optimising task allocation.
- Improved worker efficiency by minimising wasted task stealing.

## Blocking Executor

Similarly, the blocking executor makes it easy to execute blocking tasks. Use `spawn_blocking` to run tasks that may block without worrying about setting up a runtime.

- Utilizes specialized `OnceTask` for the efficient execution and scheduling of blocking tasks.
- Features an adaptable thread pool that dynamically adjusts its size as workloads change, ensuring efficient handling of varying demands.

# Usage

To use `easy-async`, add it to your Cargo.toml:

```toml
[dependencies]
easy-async = "0.1.0"
```

# Examples

The following code snippet showcases the utilization of `easy-async`'s `spawn` and `block_on` for task execution.

```rust
use std::future::ready;

fn main() {
    // Define the total number of asynchronous tasks to spawn.
    const N: usize = 1_000_000;

    // Preallocate space for storing task handles to improve performance.
    let mut handles = Vec::with_capacity(N);

    // Initialize the accumulator for the sum of the results.
    let mut sum = 0;

    // Spawn `N` asynchronous tasks, each evaluating to `1`.
    for _ in 0..N {
        handles.push(easy_async::spawn(ready(1)));
    }

    // Await the completion of each task and accumulate their results.
    for handle in handles {
        sum += easy_async::block_on(handle);
    }

    // Verify that the sum of the results equals the number of tasks.
    assert_eq!(sum, N);
}
```

The following example demonstrates the concurrent computation of Fibonacci numbers using `easy-async`'s `spawn_blocking` and `block_on`.

```rust
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
```
