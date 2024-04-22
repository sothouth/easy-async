use std::future::ready;

fn main() {
    const N: usize = 1_000_000;

    let mut handles = Vec::with_capacity(N);

    let mut sum = 0;

    for _ in 0..N {
        handles.push(easy_async::spawn(ready(1)));
    }

    for handle in handles {
        sum += easy_async::block_on(handle);
    }

    assert_eq!(sum, N);
}
