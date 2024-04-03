use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct T {
    x: UnsafeCell<i32>,
    y: UnsafeCell<i32>,
    i: AtomicUsize,
}

unsafe impl Send for T {}
unsafe impl Sync for T {}

fn main() {
    let xyi = Arc::new(T {
        x: UnsafeCell::new(0),
        y: UnsafeCell::new(0),
        i: AtomicUsize::new(0),
    });
    let th2 = thread::spawn({
        let xyi = xyi.clone();
        move || unsafe {
            xyi.i.load(Ordering::Acquire);
            // while xyi.i.load(Ordering::Acquire)==0{};
            (*xyi.y.get()) = 2;
        }
    });
    thread::sleep(Duration::from_millis(100));
    let th1 = thread::spawn({
        let xyi = xyi.clone();
        move || unsafe {
            (*xyi.x.get()) = (*xyi.y.get()) + 1;
            xyi.i.store(1, Ordering::Release);
        }
    });
    th2.join();
    th1.join();
    println!("{:?}", unsafe {
        (*xyi.x.get(), *xyi.y.get(), xyi.i.load(Ordering::Relaxed))
    });
}
