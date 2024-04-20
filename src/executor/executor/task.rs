use std::cell::UnsafeCell;
use std::fmt;
use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::executor::executor::Runtime;
use crate::waker::AtomicWaker;

mod waker;
use waker::scheduler;

pub fn task_and_handle(
    fut: impl Future<Output = ()> + Send + 'static,
    rt: &Arc<Runtime>,
) -> (Arc<Task>, TaskHandle) {
    let task = Task::new(fut, rt);
    let handle = task.handle();
    (task, handle)
}

const NORMAL: usize = 0;
const SCHEDULED: usize = 1 << 0;
const RUNNING: usize = 1 << 1;
const COMPLETED: usize = 1 << 2;
const SLEEPING: usize = 1 << 3;

pub struct Task {
    state: AtomicUsize,
    fut: UnsafeCell<Pin<Box<dyn Future<Output = ()> + Send>>>,
    rt: Arc<Runtime>,
    queue_id: AtomicUsize,
    /// can waiting for the task
    waker: AtomicWaker,
}

impl Task {
    /// Must call `TaskHandle::run` after calling this.
    pub fn new(fut: impl Future<Output = ()> + Send + 'static, rt: &Arc<Runtime>) -> Arc<Self> {
        rt.tasks.fetch_add(1, AcqRel);
        Arc::new(Self {
            state: AtomicUsize::new(SLEEPING),
            fut: UnsafeCell::new(Box::pin(fut)),
            rt: Arc::clone(rt),
            queue_id: AtomicUsize::new(rt.local_queues.len()),
            waker: AtomicWaker::new(),
        })
    }

    pub fn handle(self: &Arc<Self>) -> TaskHandle {
        TaskHandle::new(self)
    }

    pub fn schedule(self: &Arc<Self>) {
        unsafe { TaskHandle::schedule(self.as_ref()) }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("ptr", &format!("{:p}", self as *const _))
            .field(
                "state",
                &match self.state.load(Acquire) {
                    NORMAL => "NORMAL",
                    SCHEDULED => "SCHEDULED",
                    RUNNING => "RUNNING",
                    COMPLETED => "COMPLETED",
                    SLEEPING => "SLEEPING",
                    _ => "UNKNOWN",
                },
            )
            .field("queue_id", &self.queue_id)
            .finish()
    }
}

#[repr(transparent)]
pub struct TaskHandle {
    task: Arc<Task>,
}

unsafe impl Send for TaskHandle {}
unsafe impl Sync for TaskHandle {}

impl TaskHandle {
    pub fn new(task: &Arc<Task>) -> Self {
        Self {
            task: Arc::clone(task),
        }
    }

    pub fn run(self, queue_id: usize) {
        let task = self.task;

        task.state.store(RUNNING, Release);

        task.queue_id.store(queue_id, Release);

        let waker = ManuallyDrop::new(scheduler(&task));

        let mut cx = Context::from_waker(&waker);

        match unsafe { &mut *task.fut.get() }.as_mut().poll(&mut cx) {
            Poll::Ready(()) => {
                task.state.store(COMPLETED, Release);
                task.rt.tasks.fetch_sub(1, AcqRel);
                task.waker.wake();
            }
            Poll::Pending => {
                match task
                    .state
                    .compare_exchange(RUNNING, SLEEPING, AcqRel, Acquire)
                {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    }

    pub fn task(&self) -> &Arc<Task> {
        &self.task
    }

    pub unsafe fn schedule(this: *const Task) {
        let task = ManuallyDrop::new(Arc::from_raw(this));

        // Try to schedule the task from SLEEPING.
        match task
            .state
            .compare_exchange(SLEEPING, SCHEDULED, AcqRel, Acquire)
        {
            Ok(_) => {}
            Err(RUNNING) => {
                // Task has already been scheduled.
                if task
                    .state
                    .compare_exchange(RUNNING, SCHEDULED, AcqRel, Acquire)
                    .is_err()
                {
                    return;
                }
            }
            Err(_) => {
                return;
            }
        }

        let queue = task.queue_id.load(Acquire);
        let handle = Self::new(&task);

        if queue != task.rt.local_queues.len() {
            if let Err(err) = task.rt.local_queues[queue].push(handle) {
                let handle = err.into_inner();
                debug_assert!(task.rt.global_queue.push(handle).is_ok());
            }
        } else {
            debug_assert!(task.rt.global_queue.push(handle).is_ok());
        }
    }
}

impl Future for TaskHandle {
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

impl Clone for TaskHandle {
    fn clone(&self) -> Self {
        Self {
            task: Arc::clone(&self.task),
        }
    }
}

impl From<Arc<Task>> for TaskHandle {
    fn from(task: Arc<Task>) -> Self {
        Self { task }
    }
}
