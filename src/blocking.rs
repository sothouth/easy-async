use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;

use concurrent_queue::ConcurrentQueue;

mod task;

use task::{task_and_handle, OnceTaskHandle};

static GLOBAL: OnceLock<ThreadPool> = OnceLock::new();

pub fn spawn_blocking(f: impl FnOnce() + Send + 'static) -> OnceTaskHandle {
    ThreadPool::get().spawn_blocking(f)
}

const TIMEOUT: u64 = 500;

const DEFAULT_SIZE: usize = 500;

const MIN_SIZE: usize = 1;

const MAX_SIZE: usize = 10000;

const ENV_SIZE_NAME: &str = "BLOCKING_POOL_SIZE";

pub struct ThreadPool {
    inner: Mutex<Inner>,
    queue: Arc<ConcurrentQueue<OnceTaskHandle>>,
    notifier: Condvar,
}

pub struct Inner {
    idle_count: usize,
    thread_count: usize,
    pool_size: usize,
}

impl ThreadPool {
    /// Use singleton thread pool.
    #[inline]
    fn get() -> &'static Self {
        GLOBAL.get_or_init(|| {
            let pool_size = match env::var(ENV_SIZE_NAME) {
                Ok(s) => s.parse().unwrap_or(DEFAULT_SIZE),
                Err(_) => DEFAULT_SIZE,
            };

            assert!(pool_size >= MIN_SIZE);
            assert!(pool_size <= MAX_SIZE);

            Self {
                inner: Mutex::new(Inner {
                    idle_count: 0,
                    thread_count: 0,
                    pool_size,
                }),
                queue: Arc::new(ConcurrentQueue::unbounded()),
                notifier: Condvar::new(),
            }
        })
    }

    fn spawn_blocking(&'static self, f: impl FnOnce() + Send + 'static) -> OnceTaskHandle {
        let (task, handle) = task_and_handle(f);

        debug_assert!(self.queue.push(task.into()).is_ok());

        self.schedule(self.inner.lock().unwrap());

        handle
    }

    /// Start a new worker on the current thread.
    fn worker(&'static self) {
        let mut inner = self.inner.lock().unwrap();

        loop {
            inner.idle_count -= 1;

            while let Ok(task) = self.queue.pop() {
                self.schedule(inner);

                task.run();

                inner = self.inner.lock().unwrap();
            }

            inner.idle_count += 1;

            let (lock, res) = self
                .notifier
                .wait_timeout(inner, Duration::from_millis(TIMEOUT))
                .unwrap();
            inner = lock;

            if res.timed_out() && self.queue.is_empty() {
                inner.idle_count -= 1;
                inner.thread_count -= 1;
                break;
            }
        }
    }

    /// Schedule the worker and spawn more workers if necessary.
    fn schedule(&'static self, mut inner: MutexGuard<'static, Inner>) {
        self.notifier.notify_one();

        while self.queue.len() > inner.idle_count * 5 && inner.thread_count < inner.pool_size {
            self.notifier.notify_all();

            static ID: AtomicUsize = AtomicUsize::new(1);
            let id = ID.fetch_add(1, Ordering::AcqRel);

            inner.thread_count += 1;
            inner.idle_count += 1;

            if let Err(_) = thread::Builder::new()
                .name(format!("blocking-{}", id))
                .spawn(move || self.worker())
            {
                inner.thread_count -= 1;
                inner.idle_count -= 1;

                // The max size of pool is not big enough.
                // Update pool_size.
                inner.pool_size = inner.thread_count.max(1);
            }
        }
    }
}
