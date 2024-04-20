use std::{
    mem::ManuallyDrop,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
    thread::{self, Thread},
};

static VTABLE: RawWakerVTable = RawWakerVTable::new(raw_clone, raw_wake, raw_wake_by_ref, raw_drop);

fn raw_clone(ptr: *const ()) -> RawWaker {
    unsafe { Arc::increment_strong_count(ptr as *const Thread) };
    RawWaker::new(ptr, &VTABLE)
}

fn raw_wake(ptr: *const ()) {
    unsafe { Arc::from_raw(ptr as *const Thread) }.unpark();
}

fn raw_wake_by_ref(ptr: *const ()) {
    ManuallyDrop::new(unsafe { Arc::from_raw(ptr as *const Thread) }).unpark();
}

fn raw_drop(ptr: *const ()) {
    unsafe { Arc::decrement_strong_count(ptr as *const Thread) };
}

/// Creates a waker for the current thread.
pub fn current_thread_waker() -> Waker {
    thread_local! {
        static THREAD: Arc<Thread> = Arc::new(thread::current());
    }
    THREAD.with(|thread| {
        let thread = Arc::into_raw(thread.clone()) as *const ();
        unsafe { Waker::from_raw(RawWaker::new(thread, &VTABLE)) }
    })
}
