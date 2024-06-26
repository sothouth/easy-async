use std::thread::sleep;
use std::time::Duration;

use easy_async::waker::Parker;
use easy_parallel::Parallel;

#[test]
fn park_timeout_unpark_before() {
    let p = Parker::new();
    for _ in 0..10 {
        p.unparker().unpark();
        p.park_timeout(Duration::from_millis(u32::MAX as u64));
    }
}

#[test]
fn park_timeout_unpark_not_called() {
    let p = Parker::new();
    for _ in 0..10 {
        p.park_timeout(Duration::from_millis(10));
    }
}

#[test]
fn park_timeout_unpark_called_other_thread() {
    for _ in 0..10 {
        let p = Parker::new();
        let u = p.unparker();

        Parallel::new()
            .add(move || {
                sleep(Duration::from_millis(50));
                u.unpark();
            })
            .add(move || {
                p.park_timeout(Duration::from_millis(u32::MAX as u64));
            })
            .run();
    }
}
