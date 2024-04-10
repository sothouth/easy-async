use std::sync::atomic::{AtomicUsize, Ordering};

use easy_parallel::Parallel;

use easy_async::utils::xpxc::prelude::*;

#[test]
fn smoke() {
    let q = Single::new();

    q.push(7).unwrap();
    assert_eq!(q.pop(), Ok(7));

    q.push(8).unwrap();
    assert_eq!(q.pop(), Ok(8));
    assert!(q.pop().is_err());
}

#[test]
fn capacity() {
    let q = Single::<usize>::new();
    assert_eq!(q.capacity(), 1);
}

#[test]
fn len_empty_full() {
    let q = Single::new();

    assert_eq!(q.len(), 0);
    assert_eq!(q.is_empty(), true);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), true);

    q.pop().unwrap();

    assert_eq!(q.len(), 0);
    assert_eq!(q.is_empty(), true);
    assert_eq!(q.is_full(), false);
}

#[test]
fn spsc() {
    const COUNT: usize = if cfg!(miri) { 100 } else { 100_000 };

    let q = Single::new();

    Parallel::new()
        .add(|| {
            for i in 0..COUNT {
                loop {
                    if let Ok(x) = q.pop() {
                        assert_eq!(x, i);
                        break;
                    }
                }
            }
            assert!(q.pop().is_err());
        })
        .add(|| {
            for i in 0..COUNT {
                while q.push(i).is_err() {}
            }
        })
        .run();
}

#[test]
fn mpmc() {
    const COUNT: usize = if cfg!(miri) { 100 } else { 25_000 };
    const THREADS: usize = 1;

    let q = Single::<usize>::new();
    let v = (0..COUNT).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>();

    Parallel::new()
        .each(0..THREADS, |_| {
            for _ in 0..COUNT {
                let n = loop {
                    if let Ok(x) = q.pop() {
                        break x;
                    }
                };
                v[n].fetch_add(1, Ordering::SeqCst);
            }
        })
        .each(0..THREADS, |_| {
            for i in 0..COUNT {
                while q.push(i).is_err() {}
            }
        })
        .run();

    for c in v {
        assert_eq!(c.load(Ordering::SeqCst), THREADS);
    }
}

#[test]
fn drops() {
    const RUNS: usize = if cfg!(miri) { 20 } else { 100 };
    const STEPS: usize = if cfg!(miri) { 100 } else { 10_000 };

    static DROPS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, PartialEq)]
    struct DropCounter;

    impl Drop for DropCounter {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }

    for _ in 0..RUNS {
        let steps = fastrand::usize(..STEPS);
        let additional = fastrand::usize(0..=1);

        DROPS.store(0, Ordering::SeqCst);
        let q = Single::new();

        Parallel::new()
            .add(|| {
                for _ in 0..steps {
                    while q.pop().is_err() {}
                }
            })
            .add(|| {
                for _ in 0..steps {
                    while q.push(DropCounter).is_err() {
                        DROPS.fetch_sub(1, Ordering::SeqCst);
                    }
                }
            })
            .run();

        for _ in 0..additional {
            q.push(DropCounter).unwrap();
        }

        assert_eq!(DROPS.load(Ordering::SeqCst), steps);
        drop(q);
        assert_eq!(DROPS.load(Ordering::SeqCst), steps + additional);
    }
}

#[test]
fn linearizable() {
    const COUNT: usize = if cfg!(miri) { 500 } else { 25_000 };
    const THREADS: usize = 4;

    let q = Single::new();

    Parallel::new()
        .each(0..THREADS, |_| {
            for _ in 0..COUNT {
                while q.push(0).is_err() {}
                q.pop().unwrap();
            }
        })
        .run();
}
