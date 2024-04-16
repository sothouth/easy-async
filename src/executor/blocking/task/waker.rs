use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use super::Task;
use super::TaskHandle;

const VTABLE: &RawWakerVTable = &{
    #[inline]
    unsafe fn raw_clone(ptr: *const ()) -> RawWaker {
        Arc::increment_strong_count(ptr as *const Task);
        RawWaker::new(ptr, VTABLE)
    }

    #[inline]
    unsafe fn raw_wake(ptr: *const ()) {
        raw_wake_by_ref(ptr);
        raw_drop(ptr);
    }

    #[inline]
    unsafe fn raw_wake_by_ref(ptr: *const ()) {
        TaskHandle::schedule(ptr as *const Task);
    }

    #[inline]
    unsafe fn raw_drop(ptr: *const ()) {
        Arc::decrement_strong_count(ptr as *const Task);
    }

    RawWakerVTable::new(raw_clone, raw_wake, raw_wake_by_ref, raw_drop)
};

pub fn scheduler(task: &Arc<Task>) -> Waker {
    let task = Arc::into_raw(task.clone());
    unsafe { Waker::from_raw(RawWaker::new(task as *const (), VTABLE)) }
}
