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
