/// can use NOOP
use std::{
    mem::ManuallyDrop,
    ptr,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
    thread::{self, Thread},
};

static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

fn clone(_: *const ()) -> RawWaker {
    RawWaker::new(ptr::null(), &VTABLE)
}

fn wake(_: *const ()) {}

fn wake_by_ref(_: *const ()) {}

fn drop(_: *const ()) {}

pub(crate) fn empty_waker() -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &VTABLE)) }
}
