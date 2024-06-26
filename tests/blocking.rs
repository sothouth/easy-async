use std::pin::pin;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use easy_async::block_on;
use easy_async::spawn_blocking;

use easy_async::future::poll_once;

#[test]
fn smol_smoke() {
    use smol::block_on;
    use smol::unblock;

    const N: usize = 10000;
    const M: usize = 1000;
    let a = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::with_capacity(N);

    let start = std::time::Instant::now();
    for _ in 0..N {
        let a = a.clone();
        tasks.push(unblock(move || {
            for _ in 0..M {
                a.fetch_add(1, SeqCst);
            }
        }));
    }

    for t in tasks {
        block_on(t);
    }

    assert_eq!(a.load(Acquire), N * M);
    println!("smol: {}ms", start.elapsed().as_millis());
}

#[test]
fn smoke() {
    const N: usize = 10000;
    const M: usize = 1000;
    let a = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::with_capacity(N);

    let start = std::time::Instant::now();
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
    println!("easy-async: {}ms", start.elapsed().as_millis());
}

#[test]
fn sleep() {
    let dur = Duration::from_millis(300);
    let start = Instant::now();

    block_on(async {
        let f = spawn_blocking(move || thread::sleep(dur));
        let mut f = pin!(f);
        assert!(poll_once(f.as_mut()).await.is_none());
        f.await
    });
    assert!(start.elapsed() > dur);
}

#[test]
fn get_output() {
    block_on(async {
        assert_eq!(spawn_blocking(|| 1).await, 1);
    });
}
