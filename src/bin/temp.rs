use std::borrow::Borrow;
use std::cell::Cell;
use std::cell::UnsafeCell;
use std::fmt;
use std::ptr;
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

// #[no_mangle]
pub fn b() -> isize {
    let n = Arc::new(Cell::new(0));
    *n.borrow_mut() += 1;
    n.get()
}

// #[no_mangle]
pub fn r() -> isize {
    let mut n = 0;
    let rn = &mut n;
    *rn += 1;
    *rn
}

fn main() {}
