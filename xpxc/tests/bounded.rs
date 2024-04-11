#![allow(clippy::bool_assert_comparison)]

use std::sync::atomic::{AtomicUsize, Ordering};

use easy_parallel::Parallel;

use xpxc::prelude::*;

#[test]
fn smoke() {
    let q = Bounded::with_capacity(2);

    q.push(7).unwrap();
    assert_eq!(q.pop(), Ok(7));

    q.push(8).unwrap();
    assert_eq!(q.pop(), Ok(8));
    assert!(q.pop().is_err());
}

#[test]
fn capacity() {
    let q = Bounded::<usize>::with_capacity(2);
    println!("{}", q.capacity());
    for i in 1..10 {
        let q = Bounded::<i32>::with_capacity(i);
        assert_eq!(q.capacity(), i);
    }
}

#[test]
#[should_panic(expected = "capacity must be positive")]
fn zero_capacity() {
    let _ = Bounded::<i32>::with_capacity(0);
}

#[test]
fn len_empty_full() {
    let q = Bounded::with_capacity(2);

    assert_eq!(q.len(), 0);
    assert_eq!(q.is_empty(), true);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);

    q.push(()).unwrap();

    assert_eq!(q.len(), 2);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), true);

    q.pop().unwrap();

    assert_eq!(q.len(), 1);
    assert_eq!(q.is_empty(), false);
    assert_eq!(q.is_full(), false);
}

#[test]
fn len() {
    const COUNT: usize = if cfg!(miri) { 50 } else { 25_000 };
    const CAP: usize = if cfg!(miri) { 50 } else { 1000 };

    let q = Bounded::with_capacity(CAP);
    assert_eq!(q.len(), 0);

    for _ in 0..CAP / 10 {
        for i in 0..50 {
            q.push(i).unwrap();
            assert_eq!(q.len(), i + 1);
        }

        for i in 0..50 {
            q.pop().unwrap();
            assert_eq!(q.len(), 50 - i - 1);
        }
    }
    assert_eq!(q.len(), 0);

    for i in 0..CAP {
        q.push(i).unwrap();
        assert_eq!(q.len(), i + 1);
    }

    for _ in 0..CAP {
        q.pop().unwrap();
    }
    assert_eq!(q.len(), 0);

    Parallel::new()
        .add(|| {
            for i in 0..COUNT {
                loop {
                    if let Ok(x) = q.pop() {
                        assert_eq!(x, i);
                        break;
                    }
                }
                let len = q.len();
                assert!(len <= CAP);
            }
        })
        .add(|| {
            for i in 0..COUNT {
                while q.push(i).is_err() {}
                let len = q.len();
                assert!(len <= CAP);
            }
        })
        .run();

    assert_eq!(q.len(), 0);
}


#[test]
fn spsc() {
    const COUNT: usize = if cfg!(miri) { 100 } else { 100_000 };

    let q = Bounded::with_capacity(3);

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
    const THREADS: usize = 4;

    let q = Bounded::<usize>::with_capacity(3);
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
    const RUNS: usize = if cfg!(miri) { 10 } else { 100 };
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
        let additional = fastrand::usize(..50);

        DROPS.store(0, Ordering::SeqCst);
        let q = Bounded::with_capacity(50);

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

    let q = Bounded::with_capacity(THREADS);

    Parallel::new()
        .each(0..THREADS, |_| {
            for _ in 0..COUNT {
                while q.push(0).is_err() {}
                q.pop().unwrap();
            }
        })
        .run();
}