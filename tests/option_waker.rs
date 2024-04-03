#![feature(noop_waker)]
use easy_async::task::option_waker::OptionWaker;
use easy_async::task::thread_waker::current_thread_waker;
use std::task::Waker;

#[test]
fn same_check() {
    let a = OptionWaker::new();
    let b = Waker::noop().clone();
    assert!(a.will_wake(&b));
    let c = OptionWaker::new();
    assert!(a.will_wake(&c));
    let d = a.clone();
    assert!(a.will_wake(&d));

    let e = current_thread_waker();
    assert!(!a.will_wake(&e));
    let f = current_thread_waker();
    assert!(!e.will_wake(&f));
    let e = e.clone();
    assert!(e.will_wake(&e));
}
