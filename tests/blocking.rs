use easy_async::block_on;
use easy_async::blocking::spawn_blocking;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::Arc;

#[test]
fn smoke() {
    const N: usize = 10;
    const M: usize = 1000;
    let a = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::with_capacity(N);

    for _ in 0..N {
        let a = a.clone();
        tasks.push(spawn_blocking(move || {
            for _ in 0..M {
                a.fetch_add(1, SeqCst);
            }
        }));
    }

    for t in tasks {
        block_on(t);
    }

    assert_eq!(a.load(Acquire), N * M);
}
