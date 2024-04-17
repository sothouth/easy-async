use std::cell::UnsafeCell;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering::*};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fmt, ptr};

use crate::waker::AtomicWaker;

/// # Note
///
///  OnceTask is assumed to be scheduled when it is constructed.
pub fn task_and_handle(fut: impl FnOnce() + Send + 'static) -> (Arc<OnceTask>, OnceTaskHandle) {
    let task = OnceTask::new(fut);
    let handle = task.handle();
    (task, handle)
}

const SCHEDULED: usize = 1 << 0;
const RUNNING: usize = 1 << 1;
const COMPLETED: usize = 1 << 2;

pub struct OnceTask {
    state: AtomicUsize,
    f: UnsafeCell<ManuallyDrop<Box<dyn FnOnce() + Send>>>,
    /// can waiting for the task
    waker: AtomicWaker,
}

impl OnceTask {
    /// Must call `TaskHandle::run` after calling this.
    pub fn new(f: impl FnOnce() + Send + 'static) -> Arc<Self> {
        Arc::new(Self {
            state: AtomicUsize::new(SCHEDULED),
            f: UnsafeCell::new(ManuallyDrop::new(Box::new(f))),
            waker: AtomicWaker::new(),
        })
    }

    pub fn handle(self: &Arc<Self>) -> OnceTaskHandle {
        OnceTaskHandle::new(self)
    }
}

impl Drop for OnceTask {
    fn drop(&mut self) {
        // Drop the task if it is not run.
        if self.state.load(Acquire) != COMPLETED {
            let _ = unsafe { Box::from_raw(self.f.get()) };
        }
    }
}

impl fmt::Debug for OnceTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("ptr", &format!("{:p}", self as *const _))
            .field(
                "state",
                &match self.state.load(Acquire) {
                    SCHEDULED => "SCHEDULED",
                    RUNNING => "RUNNING",
                    COMPLETED => "COMPLETED",
                    _ => "UNKNOWN",
                },
            )
            .finish()
    }
}

#[repr(transparent)]
pub struct OnceTaskHandle {
    task: Arc<OnceTask>,
}

unsafe impl Send for OnceTaskHandle {}
unsafe impl Sync for OnceTaskHandle {}

impl OnceTaskHandle {
    pub fn new(task: &Arc<OnceTask>) -> Self {
        Self {
            task: Arc::clone(task),
        }
    }

    pub fn run(self) {
        let task = self.task;

        if task
            .state
            .compare_exchange(SCHEDULED, RUNNING, AcqRel, Acquire)
            .is_err()
        {
            panic!("double run.");
            return;
        }

        unsafe {
            let closure_ptr = task.f.get();
            let closure = ptr::read(closure_ptr);

            ManuallyDrop::into_inner(closure)();
        }

        task.state.store(COMPLETED, Release);

        task.waker.wake();
    }

    pub fn task(&self) -> &Arc<OnceTask> {
        &self.task
    }
}

impl Future for OnceTaskHandle {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.task.state.load(Acquire) == COMPLETED {
            Poll::Ready(())
        } else {
            self.task.waker.register(cx.waker());
            Poll::Pending
        }
    }
}

impl Clone for OnceTaskHandle {
    fn clone(&self) -> Self {
        Self {
            task: Arc::clone(&self.task),
        }
    }
}

impl From<Arc<OnceTask>> for OnceTaskHandle {
    fn from(task: Arc<OnceTask>) -> Self {
        Self { task }
    }
}
