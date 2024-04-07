//! poor imitation of async-executor
use std::cell::UnsafeCell;
use std::future::{poll_fn, Future};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::task::{Poll, Waker};
use std::thread;

use concurrent_queue::ConcurrentQueue;
use slab::Slab;

use async_task::{Builder as TaskBuilder, Runnable, Task};
use async_lock::OnceCell;

use crate::utils::call_on_drop::CallOnDrop;
use crate::waker::OptionWaker;

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

                        crate::block_on(async { ex.working(ith).await });
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

    pub async fn working(&self, id: usize) {
        let worker = Worker::new(id, &self.rt);

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
            rt.notify();
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
    searching: AtomicBool,
    /// Sleeping Worker's wakers.
    waiters: Mutex<Waiters>,
    /// All task's waker.
    tasks: Mutex<Slab<Waker>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_queue: ConcurrentQueue::unbounded(),
            local_queues: RwLock::new(Vec::new()),
            searching: AtomicBool::new(true),
            waiters: Mutex::new(Waiters::new()),
            tasks: Mutex::new(Slab::new()),
        }
    }

    pub fn notify(&self) {
        match self
            .searching
            .compare_exchange(false, true, AcqRel, Acquire)
        {
            // no worker is searching
            Ok(_) => {
                if let Some(waker) = self.waiters.lock().unwrap().wake() {
                    waker.wake();
                }
            }
            // already have a worker is searching
            Err(_) => {}
        }
    }
}

pub struct Waiters {
    len: usize,
    wakers: Vec<(usize, Waker)>,
    free_ids: Vec<usize>,
}

impl Waiters {
    pub fn new() -> Self {
        Self {
            len: 0,
            wakers: Vec::new(),
            free_ids: Vec::new(),
        }
    }

    /// Register a waker and return its id.
    pub fn insert(&mut self, waker: &Waker) -> usize {
        let id = self.free_ids.pop().unwrap_or(self.len);
        self.len += 1;
        self.wakers.push((id, waker.clone()));
        id
    }

    /// Re-insert a waker if it was waked.
    ///
    /// use `clone_from` to update the waker if it was not waked.
    ///
    /// Returns `true` if the waker was waked.
    ///
    /// Returns `false` if the waker is still in the wakers list.
    pub fn update(&mut self, id: usize, waker: &Waker) -> bool {
        for (cur_id, cur_waker) in self.wakers.iter_mut() {
            if *cur_id == id {
                cur_waker.clone_from(waker);
                return false;
            }
        }

        self.wakers.push((id, waker.clone()));
        true
    }

    /// Deregister a id whether it is waked or not.
    ///
    /// Returns `true` if the waker is the searching worker.
    ///
    /// Returns `false` if the waker was still sleeping.
    pub fn remove(&mut self, id: usize) -> bool {
        self.len -= 1;
        self.free_ids.push(id);

        for i in (0..self.wakers.len()).rev() {
            if self.wakers[i].0 == id {
                self.wakers.remove(i);
                // self.wakers.swap_remove(i);
                return false;
            }
        }
        true
    }

    /// There is max one waker will at waked state at any time.
    ///
    /// Return true if there is a waker was waked.
    #[inline]
    pub fn is_waked(&self) -> bool {
        self.len == 0 || self.len > self.wakers.len()
    }

    /// Reture a waker if there is one and not `is_waked`.
    ///
    /// Note that the waker is just remove from the wakers list,
    /// should call `remove` to deregister it.
    pub fn wake(&mut self) -> Option<Waker> {
        if self.wakers.len() == self.len {
            self.wakers.pop().map(|(_, waker)| waker)
        } else {
            None
        }
    }
}

const LOCAL_QUEUE_SIZE: usize = 512;

/// single thread worker
pub struct Worker<'a> {
    /// The worker's unique id.
    id: usize,
    /// The runtime.
    rt: &'a Runtime,
    /// The worker's task queue.
    queue: Arc<ConcurrentQueue<Runnable>>,
    /// When 0,the worker is working, when >0, the worker is sleeping.
    sleeping: AtomicUsize,
    /// The times this worker polls.
    ticks: AtomicUsize,
    /// Next steal worker.
    next: AtomicUsize,
}

impl<'a> Worker<'a> {
    pub fn new(id: usize, rt: &'a Runtime) -> Self {
        let queue = Arc::new(ConcurrentQueue::bounded(LOCAL_QUEUE_SIZE));

        rt.local_queues.write().unwrap().push(queue.clone());

        Self {
            id,
            rt,
            queue,
            sleeping: AtomicUsize::new(0),
            ticks: AtomicUsize::new(0),
            next: AtomicUsize::new(id),
        }
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

    pub async fn next(&self) -> Runnable {
        let runnable = self
            .next_by(|| {
                // Get tast from self queue.
                if let Ok(r) = self.queue.pop() {
                    return Some(r);
                }

                // Get task from global_queue.
                if let Ok(r) = self.rt.global_queue.pop() {
                    steal(&self.rt.global_queue, &self.queue);
                    return Some(r);
                }

                // Get task from other workers.
                let local_queues = self.rt.local_queues.read().unwrap();
                let n = local_queues.len();

                let mut from = (self.id + 1) % n;

                while from != self.id {
                    steal(&local_queues[from], &self.queue);
                    if let Ok(r) = self.queue.pop() {
                        return Some(r);
                    }
                    from = (from + 1) % n;
                }

                None
            })
            .await;

        let ticks = self.ticks.fetch_add(1, AcqRel);
        if ticks % 128 == 0 {
            steal(&self.rt.global_queue, &self.queue);
        }

        runnable
    }

    pub async fn next_by(&self, mut search: impl FnMut() -> Option<Runnable>) -> Runnable {
        poll_fn(|cx| loop {
            match search() {
                Some(r) => {
                    // This worker have tasks to run.
                    self.wake();

                    // Wake up a other worker.
                    self.rt.notify();

                    return Poll::Ready(r);
                }
                None => {
                    // Should sleep.
                    if !self.sleep(cx.waker()) {
                        return Poll::Pending;
                    }
                }
            }
        })
        .await
    }

    /// `true` means it should keep running.
    /// * The worker is just searching.
    /// * The worker is being woken.
    ///
    /// `false` means it is not the searching worker, and should sleep.
    pub fn sleep(&self, waker: &Waker) -> bool {
        let mut waiters = self.rt.waiters.lock().unwrap();

        match self.sleeping.load(Acquire) {
            // The worker was not sleeping,
            // register it.
            0 => {
                self.sleeping.store(waiters.insert(waker), Release);
            }
            // The worker was sleeping.
            id => {
                // True means the waker was pop by `Waiter::wake`,
                // false means there was some wrong, should sleep again.
                if !waiters.update(id, waker) {
                    return false;
                }
            }
        }

        self.rt.searching.store(waiters.is_waked(), Release);

        true
    }

    /// Rerun the worker.
    pub fn wake(&self) {
        let id = self.sleeping.swap(0, AcqRel);
        if id != 0 {
            let mut waiters = self.rt.waiters.lock().unwrap();
            waiters.remove(id);

            self.rt.searching.swap(waiters.is_waked(), Release);
        }
    }
}

impl Drop for Worker<'_> {
    fn drop(&mut self) {
        // Remove the worker form the waiters list.
        let id = self.sleeping.swap(0, AcqRel);
        if id != 0 {
            let mut waiters = self.rt.waiters.lock().unwrap();
            let searching = waiters.remove(id);

            self.rt.searching.swap(waiters.is_waked(), Release);

            // If self was the searching worker, wake up a other worker.
            if searching {
                drop(waiters);
                self.rt.notify();
            }
        }

        // Remove the worker's queue and reschedule all tasks to the globle_queue.
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
    for _ in 0..((src.len() + 1) / 2).min(dst.capacity().unwrap() - dst.len()) {
        match src.pop() {
            Ok(r) => assert!(dst.push(r).is_ok()),
            Err(_) => break,
        }
    }
}
