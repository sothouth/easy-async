//! No output executor
//!
//! In \tests\executor.rs: no_output_many_async 10 times faster than smol_many_async
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use concurrent_queue::ConcurrentQueue;

mod task;
use task::{task_and_handle, Task, TaskHandle};

static GLOBAL: OnceLock<Executor> = OnceLock::new();

/// spawn a task
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where
    F: Future<Output = T> + Send + 'static,
{
    GLOBAL.get_or_init(Executor::new).spawn(future)
}

pub fn complated() -> bool {
    GLOBAL.get_or_init(Executor::new).is_empty()
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

    pub fn spawn<F, T>(&self, future: F) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
    {
        let (task, handle) = task_and_handle(future, &self.rt);

        task.schedule();

        handle
    }

    pub fn state(&self) -> usize {
        self.rt.tasks.load(Acquire)
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
    pub(crate) global_queue: ConcurrentQueue<Task>,
    /// All worker's queues.
    pub(crate) local_queues: Vec<Arc<ConcurrentQueue<Task>>>,
    /// Keep at most one worker in a loop search with None as the answer.
    pub(crate) searching: Mutex<()>,
    /// Counter for task number.
    pub(crate) tasks: AtomicUsize,
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
    queue: Arc<ConcurrentQueue<Task>>,
    /// If true, the worker can loop search with None as the answer.
    searching: AtomicBool,
    /// The times this worker polls.
    ticks: AtomicUsize,
}

impl Worker {
    pub fn new(id: usize, rt: Arc<Runtime>, queue: Arc<ConcurrentQueue<Task>>) -> Self {
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

    pub fn next(&self) -> Task {
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
        if ticks % 512 == 0 {
            // println!("{} {}", self.id, ticks);
            steal(&self.rt.global_queue, &self.queue, &self.rt.global_queue);
        }

        task
    }

    pub fn next_by(&self, mut search: impl FnMut() -> Option<Task>) -> Task {
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
            Ok(task) => {
                if let Err(task) = dst.push(task) {
                    let task = task.into_inner();
                    if let Err(task) = src.push(task) {
                        let task = task.into_inner();
                        debug_assert!(global.push(task).is_ok());
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }
}
