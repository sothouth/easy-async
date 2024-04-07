//! Faster than smol.
//! * Cancel the use of asynchrony at the executor level.
//! * Generate runtime using a top-down construction approach.
//! * Removed the Sleepers mechanism with high consumption and used a simple Mutex to ensure low concurrency.
//! * Cancel random searching start, use fixed by id.
use std::cell::UnsafeCell;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock};
use std::task::Waker;
use std::thread;

use concurrent_queue::ConcurrentQueue;
use slab::Slab;

use async_task::{Builder as TaskBuilder, Runnable, Task};

use crate::utils::call_on_drop::CallOnDrop;

static GLOBAL: OnceLock<Executor<'_>> = OnceLock::new();

/// spawn a task
pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    GLOBAL.get_or_init(Executor::new).spawn(future)
}

/// handle for runtime
pub struct Executor<'a> {
    rt: Arc<Runtime>,
    /// Help to keep future and output lifetime check.
    _marker: PhantomData<UnsafeCell<&'a ()>>,
}

unsafe impl Send for Executor<'_> {}
unsafe impl Sync for Executor<'_> {}

impl<'a> Executor<'a> {
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

        Self {
            rt,
            _marker: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rt.tasks.lock().unwrap().is_empty()
    }

    pub fn spawn<T: Send + 'a>(&self, future: impl Future<Output = T> + Send + 'a) -> Task<T> {
        let mut tasks = self.rt.tasks.lock().unwrap();

        let entry = tasks.vacant_entry();
        let key = entry.key();
        let rt = self.rt.clone();
        let wraped = async move {
            let _guard = CallOnDrop(move || {
                drop(rt.tasks.lock().unwrap().try_remove(key));
            });
            future.await
        };

        let (runnable, task) = unsafe {
            TaskBuilder::new()
                .propagate_panic(true)
                .spawn_unchecked(|()| wraped, self.schedule())
        };

        entry.insert(runnable.waker());

        runnable.schedule();
        task
    }

    #[inline]
    pub fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let rt = self.rt.clone();

        move |runnable| {
            rt.global_queue.push(runnable).unwrap();
        }
    }
}

impl Drop for Executor<'_> {
    fn drop(&mut self) {
        let mut tasks = self.rt.tasks.lock().unwrap();
        for task in tasks.drain() {
            task.wake();
        }
        drop(tasks);

        while self.rt.global_queue.pop().is_ok() {}

        for queue in self.rt.local_queues.iter() {
            while queue.pop().is_ok() {}
        }
    }
}

/// runtime shared core
pub struct Runtime {
    /// Global queue, all task will be sceduled in it.
    global_queue: ConcurrentQueue<Runnable>,
    /// All worker's queues.
    local_queues: Vec<Arc<ConcurrentQueue<Runnable>>>,
    /// Keep at most one worker in a loop search with None as the answer.
    searching: Mutex<()>,
    /// All task's waker.
    tasks: Mutex<Slab<Waker>>,
}

const LOCAL_QUEUE_SIZE: usize = 512;

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_queue: ConcurrentQueue::unbounded(),
            local_queues: vec![
                Arc::new(ConcurrentQueue::bounded(LOCAL_QUEUE_SIZE));
                num_cpus::get()
            ],
            searching: Mutex::new(()),
            tasks: Mutex::new(Slab::new()),
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
    queue: Arc<ConcurrentQueue<Runnable>>,
    /// If true, the worker can loop search with None as the answer.
    searching: AtomicBool,
    /// The times this worker polls.
    ticks: AtomicUsize,
}

impl Worker {
    pub fn new(id: usize, rt: Arc<Runtime>, queue: Arc<ConcurrentQueue<Runnable>>) -> Self {
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
            let runnable = self.next();
            // println!("{} run", self.id);
            runnable.run();
        }
    }

    pub fn next(&self) -> Runnable {
        let runnable = self.next_by(|| {
            // Get tast from self queue.
            if let Ok(r) = self.queue.pop() {
                return Some(r);
            }

            // Get task from global_queue.
            if let Ok(r) = self.rt.global_queue.pop() {
                // println!("{} try", self.id);
                steal(&self.rt.global_queue, &self.queue);
                return Some(r);
            }

            // Get task from other workers.
            let local_queues = &self.rt.local_queues;

            let n = local_queues.len();
            let froms = local_queues
                .iter()
                .chain(local_queues.iter())
                .skip(self.id)
                .take(n)
                .filter(|from| Arc::ptr_eq(from, &self.queue));

            for from in froms {
                steal(from, &self.queue);
                if let Ok(r) = self.queue.pop() {
                    return Some(r);
                }
            }

            None
        });

        let ticks = self.ticks.fetch_add(1, AcqRel);
        if ticks % 128 == 0 {
            steal(&self.rt.global_queue, &self.queue);
        }

        runnable
    }

    pub fn next_by(&self, mut search: impl FnMut() -> Option<Runnable>) -> Runnable {
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

fn steal<T>(src: &ConcurrentQueue<T>, dst: &ConcurrentQueue<T>) {
    for _ in 0..((src.len() + 1) / 2).min(dst.capacity().unwrap() - dst.len()) {
        match src.pop() {
            Ok(r) => assert!(dst.push(r).is_ok()),
            Err(_) => break,
        }
    }
}
