//! poor imitation of async-executor
use std::cell::UnsafeCell;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::task::Waker;
use std::thread;

use concurrent_queue::ConcurrentQueue;
use slab::Slab;

use async_task::{Builder as TaskBuilder, Runnable, Task};

use crate::utils::call_on_drop::CallOnDrop;

static GLOBAL: OnceLock<Executor<'_>> = OnceLock::new();

/// spawn a task
pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    GLOBAL
        .get_or_init(|| {
            let ex = Executor::new();
            let num = num_cpus::get();

            for ith in 1..=num {
                thread::Builder::new()
                    .name(format!("worker-{}", ith))
                    .spawn(move || {
                        while GLOBAL.get().is_none() {
                            thread::yield_now();
                        }

                        let ex = GLOBAL.get().unwrap();

                        crate::block_on(async { ex.working().await });
                    })
                    .expect("spawn worker thread error");
            }
            ex
        })
        .spawn(future)
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
        Self {
            rt: Arc::new(Runtime::new()),
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

    pub async fn working(&self) {
        let worker = Worker::new(&self.rt);

        loop {
            let runnable = worker.next().await;
            runnable.run();
        }
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
    }
}

/// runtime core
pub struct Runtime {
    /// Global queue, all task will be sceduled in it.
    global_queue: ConcurrentQueue<Runnable>,
    /// All worker's queues.
    local_queues: RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>,
    /// If a new task is scheduled.
    new_task: AtomicBool,
    /// Sleeping Worker's wakers.
    sleepings: Mutex<Slab<Waker>>,
    /// All task's waker.
    tasks: Mutex<Slab<Waker>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_queue: ConcurrentQueue::unbounded(),
            local_queues: RwLock::new(Vec::new()),
            new_task: AtomicBool::new(true),
            sleepings: Mutex::new(Slab::new()),
            tasks: Mutex::new(Slab::new()),
        }
    }
}

const LOCAL_QUEUE_SIZE: usize = 512;

/// single thread worker
pub struct Worker<'a> {
    rt: &'a Runtime,
    queue: Arc<ConcurrentQueue<Runnable>>,
    sleeping: AtomicUsize,
}

impl<'a> Worker<'a> {
    pub fn new(rt: &'a Runtime) -> Self {
        let queue = Arc::new(ConcurrentQueue::bounded(LOCAL_QUEUE_SIZE));

        rt.local_queues.write().unwrap().push(queue.clone());

        Self {
            rt,
            queue,
            sleeping: AtomicUsize::new(0),
        }
    }

    pub async fn next(&self) -> Runnable {
        if let Ok(r) = self.queue.pop() {
            return r;
        }

        if let Ok(r) = self.rt.global_queue.pop() {
            todo!()
        }
        todo!()
    }

    pub fn block_run(&self) {
        use crate::executor::block_on::block_on;
        block_on(async { self.run().await });
    }

    pub async fn run(&self) {
        loop {
            let runnable = self.next().await;
            runnable.run();
        }
    }
}

impl Drop for Worker<'_> {
    fn drop(&mut self) {
        self.queue.close();

        self.rt
            .local_queues
            .write()
            .unwrap()
            .retain(|q| !Arc::ptr_eq(q, &self.queue));

        while let Ok(r) = self.queue.pop() {
            r.schedule();
        }
    }
}

fn steal<T>(src: &ConcurrentQueue<T>, dst: &ConcurrentQueue<T>) {
    for _ in 0..(src.len() / 2).min(dst.capacity().unwrap() - dst.len()) {
        match src.pop() {
            Ok(r) => assert!(dst.push(r).is_ok()),
            Err(_) => break,
        }
    }
}
