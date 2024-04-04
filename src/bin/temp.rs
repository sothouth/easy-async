use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::fmt;
use std::ptr;
use std::task::{RawWaker, RawWakerVTable, Waker};

// const NOOP: Waker = unsafe { Waker::from_raw(RAW_NOOP) };
// const RAW_NOOP: RawWaker = {
//     const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW_NOOP, |_| {}, |_| {}, |_| {});
//     RawWaker::new(ptr::null(), &VTABLE)
// };

// const NOOP: &Waker = &unsafe {
//     Waker::from_raw({
//         const VTABLE: RawWakerVTable = RawWakerVTable::new(|_| RAW_NOOP, |_| {}, |_| {}, |_| {});
//         RawWaker::new(ptr::null(), &VTABLE)
//     })
// };

fn main() {}
