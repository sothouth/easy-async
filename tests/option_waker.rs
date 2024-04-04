#![feature(noop_waker)]
use easy_async::task::option_waker::OptionWaker;
use easy_async::task::thread_waker::current_thread_waker;
use std::task::Waker;
use std::thread;

#[test]
fn same_check() {
    let a = OptionWaker::new();
    let b = Waker::noop().clone();
    assert!(a.will_wake(&b));

    // ?????
    // let b = Waker::noop();
    // assert!(a.will_wake(&b));

    let c = OptionWaker::new();
    assert!(a.will_wake(&c));
    let d = a.clone();
    assert!(a.will_wake(&d));

    let e = current_thread_waker();
    assert!(!a.will_wake(&e));
    let f = current_thread_waker();
    assert!(e.will_wake(&f));
    let e = e.clone();
    assert!(e.will_wake(&e));

    let f=thread::spawn(||{current_thread_waker()}).join().unwrap();
    assert!(!e.will_wake(&f));
}

#[test]
fn register() {
    let a = OptionWaker::new();
    let b = current_thread_waker();
    a.register(&b);
    assert!(a.will_wake(&b));
    let c = current_thread_waker();
    a.register(&c);
    assert!(a.will_wake(&c));
}

#[test]
fn is_noop() {
    // ????????
    // let a = OptionWaker::new();
    // assert!(a.is_noop());
    // a.register(Waker::noop());
    // assert!(a.is_noop());
}
