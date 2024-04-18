// use std::sync::atomic::AtomicUsize;
// use std::sync::atomic::Ordering::*;
// use std::task::Waker;

// use crate::waker::AtomicWaker;

// use super::raw_task::TaskVTable;

// // Task's state

// /// Task is scheduled.
// const SCHEDULED: usize = 1 << 0;
// /// Task is running.
// const RUNNING: usize = 1 << 1;
// /// Task is normal.
// const SLEEPING: usize = 1 << 2;
// /// Task is completed.
// ///
// /// The future is end.
// const COMPLETED: usize = 1 << 3;
// /// Task is closed.
// ///
// /// The future is end and the output is taken.
// const CLOSED: usize = 1 << 4;

// // Task's reference count

// /// Task's reference count one.
// const REFERENCE: usize = 1 << 5;

// pub(crate) struct Header {
//     pub(crate) state: AtomicUsize,
//     /// Notify when the task is completed.
//     pub(crate) notifier: AtomicWaker,
//     pub(crate) vtable: &'static TaskVTable,
// }

// impl Header {
//     pub(crate) fn new(vtable: &'static TaskVTable) -> Self {
//         Self {
//             state: AtomicUsize::new(SLEEPING),
//             notifier: AtomicWaker::new(),
//             vtable,
//         }
//     }

//     pub(crate) fn notify(&self) {
//         self.notifier.wake();
//     }

//     pub(crate) fn take(&self) -> Waker {
//         self.notifier.take()
//     }

//     pub(crate) fn register(&self, waker: &Waker) {
//         self.notifier.register(waker);
//     }

//     pub(crate) fn increment_ref_count(&self) {
//         self.state.fetch_add(REFERENCE, AcqRel);
//     }

//     pub(crate) fn decrement_ref_count(&self) {
//         self.state.fetch_sub(REFERENCE, AcqRel);
//     }
// }
