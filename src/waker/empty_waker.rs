//! can use [`std::task::Waker::noop`] instead
use std::ptr;
use std::task::{RawWaker, RawWakerVTable, Waker};

pub static NOOP: &Waker = &unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &NOOP_VTABLE)) };

pub static NOOP_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

fn clone(_: *const ()) -> RawWaker {
    RawWaker::new(ptr::null(), &NOOP_VTABLE)
}

fn wake(_: *const ()) {}

fn wake_by_ref(_: *const ()) {}

fn drop(_: *const ()) {}
