use std::{
    mem::ManuallyDrop,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
    thread::{self, Thread},
};

static VTABLE: RawWakerVTable = RawWakerVTable::new(raw_clone, raw_wake, raw_wake_by_ref, raw_drop);

fn raw_clone(ptr: *const ()) -> RawWaker {
    unsafe { Arc::increment_strong_count(ptr) };
    RawWaker::new(ptr, &VTABLE)
}

fn raw_wake(ptr: *const ()) {
    unsafe { Arc::from_raw(ptr as *const Thread) }.unpark();
}

fn raw_wake_by_ref(ptr: *const ()) {
    ManuallyDrop::new(unsafe { Arc::from_raw(ptr as *const Thread) }).unpark();
}

fn raw_drop(ptr: *const ()) {
    unsafe { Arc::decrement_strong_count(ptr) };
}

pub(crate) fn current_thread_waker() -> Waker {
    let thread = Arc::into_raw(Arc::new(thread::current()));
    unsafe { Waker::from_raw(RawWaker::new(thread as *const (), &VTABLE)) }
}
