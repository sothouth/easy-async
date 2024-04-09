//! No output executor
//!
//! Very fast
//!
//! In \tests\executor.rs: no_output_many_async 10 times faster than smol_many_async
use std::cell::UnsafeCell;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::pin::{pin, Pin};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::{fmt, thread};

use concurrent_queue::ConcurrentQueue;

use crate::waker::AtomicWaker;

static GLOBAL: OnceLock<Executor> = OnceLock::new();

/// spawn a task
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) -> TaskHandle {
    GLOBAL.get_or_init(Executor::new).spawn(future)
}

pub fn complated() -> bool {
    GLOBAL.get_or_init(Executor::new).is_empty()
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

pub fn task_and_handle(
    fut: impl Future<Output = ()> + Send + 'static,
    rt: &Arc<Runtime>,
) -> (Arc<Task>, TaskHandle) {
    let task = Task::new(fut, rt);
    let handle = task.handle();
    (task, handle)
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
                task.state.store(SLEEPING, Release);
            }
        }
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
                assert!(task.rt.global_queue.push(handle).is_ok());
            }
        } else {
            debug_assert!(task.rt.global_queue.push(handle).is_ok());
        }
    }
}

impl Future for TaskHandle {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.task.waker.register(cx.waker());
        if self.task.state.load(Acquire) == COMPLETED {
            Poll::Ready(())
        } else {
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

pub struct Executor {
    rt: Arc<Runtime>,
}

unsafe impl Send for Executor {}
unsafe impl Sync for Executor {}

impl Executor {
    pub fn new() -> Self {
        let rt = Arc::new(Runtime::new());

        // spawn workers fixedly
        for (ith, queue) in rt.local_queues.iter().enumerate() {
            thread::Builder::new()
                .name(format!("worker-{}", ith))
                .spawn({
                    let queue = queue.clone();
                    let rt = rt.clone();
                    move || {
                        Worker::new(ith, rt, queue).block_run();
                    }
                })
                .expect("Spawn worker thread error.");
        }

        Self { rt }
    }

    pub fn is_empty(&self) -> bool {
        self.rt.tasks.load(Acquire) == 0
    }

    pub fn spawn(&self, future: impl Future<Output = ()> + Send + 'static) -> TaskHandle {
        let (task, handle) = task_and_handle(future, &self.rt);
        task.schedule();
        handle
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        while self.rt.global_queue.pop().is_ok() {}

        for queue in self.rt.local_queues.iter() {
            while queue.pop().is_ok() {}
        }
    }
}

pub struct Runtime {
    /// Global queue, all task will be sceduled in it.
    global_queue: ConcurrentQueue<TaskHandle>,
    /// All worker's queues.
    local_queues: Vec<Arc<ConcurrentQueue<TaskHandle>>>,
    /// Keep at most one worker in a loop search with None as the answer.
    searching: Mutex<()>,
    /// Counter for task number.
    tasks: AtomicUsize,
}

const LOCAL_QUEUE_SIZE: usize = 512;
const RESERVE_SIZE: usize = 64;

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_queue: ConcurrentQueue::unbounded(),
            local_queues: (0..num_cpus::get())
                .map(|_| Arc::new(ConcurrentQueue::bounded(LOCAL_QUEUE_SIZE)))
                .collect(),
            searching: Mutex::new(()),
            tasks: AtomicUsize::new(0),
        }
    }
}

/// single thread worker
pub struct Worker {
    /// The worker's unique id.
    id: usize,
    /// The runtime.
    rt: Arc<Runtime>,
    /// The worker's task queue.
    queue: Arc<ConcurrentQueue<TaskHandle>>,
    /// If true, the worker can loop search with None as the answer.
    searching: AtomicBool,
    /// The times this worker polls.
    ticks: AtomicUsize,
}

impl Worker {
    pub fn new(id: usize, rt: Arc<Runtime>, queue: Arc<ConcurrentQueue<TaskHandle>>) -> Self {
        Self {
            id,
            rt,
            queue,
            searching: AtomicBool::new(true),
            ticks: AtomicUsize::new(0),
        }
    }

    pub fn block_run(&self) {
        loop {
            let task = self.next();
            task.run(self.id);
        }
    }

    pub fn next(&self) -> TaskHandle {
        let task = self.next_by(|| {
            // Get tast from self queue.
            if let Ok(t) = self.queue.pop() {
                return Some(t);
            }

            // Get task from global_queue.
            if let Ok(t) = self.rt.global_queue.pop() {
                steal(&self.rt.global_queue, &self.queue, &self.rt.global_queue);
                return Some(t);
            }

            // Get task from other workers.
            let local_queues = &self.rt.local_queues;

            let n = local_queues.len();
            let froms = local_queues
                .iter()
                .chain(local_queues.iter())
                .skip(self.id + 1)
                .take(n - 1);

            for from in froms {
                steal(from, &self.queue, &self.rt.global_queue);
                if let Ok(t) = self.queue.pop() {
                    return Some(t);
                }
            }

            None
        });

        let ticks = self.ticks.fetch_add(1, AcqRel);
        // if ticks % 128 == 0 {
        //     steal(&self.rt.global_queue, &self.queue, &self.rt.global_queue);
        // }

        task
    }

    pub fn next_by(&self, mut search: impl FnMut() -> Option<TaskHandle>) -> TaskHandle {
        self.searching.store(true, Release);
        loop {
            let mut _token = None;
            if self.searching.load(Acquire) == false {
                _token = Some(self.rt.searching.lock().unwrap());
            }
            match search() {
                Some(r) => {
                    return r;
                }
                None => {
                    // Should sleep.
                    self.searching.store(false, Release);
                }
            }
        }
    }
}

fn steal<T>(src: &ConcurrentQueue<T>, dst: &ConcurrentQueue<T>, global: &ConcurrentQueue<T>) {
    for _ in 0..((src.len() + 1) / 2).min(dst.capacity().unwrap() - dst.len() - RESERVE_SIZE) {
        match src.pop() {
            Ok(r) => {
                if let Err(task) = dst.push(r) {
                    let task = task.into_inner();
                    debug_assert!(global.push(task).is_ok());
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
