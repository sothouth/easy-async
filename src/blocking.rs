//! A thread pool. Used to execute blocking code in an async environment.
//!
//! # Examples
//!
//! ```no_run
//! use easy_async::spawn_blocking;
//!
//! spawn_blocking(|| println!("now running on a worker thread")).await;
//! ```

use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::Duration;

use concurrent_queue::ConcurrentQueue;

mod once_task;

use once_task::{task_and_handle, OnceTask};

pub use once_task::OnceTaskHandle;

/// Runs blocking code on a thread pool.
///
/// # Example
///
///
/// ```no_run
/// use easy_async::spawn_blocking;
///
/// assert_eq!(
///     spawn_blocking(|| {
///         println!("now running on a worker thread");
///         1
///     }).await,
///     1
/// );
/// ```
pub fn spawn_blocking<F, T>(f: F) -> OnceTaskHandle<T>
where
    F: FnOnce() -> T,
{
    ThreadPool::get().spawn_blocking(f)
}

/// Default size of thread pool.
const DEFAULT_SIZE: usize = 500;

/// Minimum size of thread pool.
const MIN_SIZE: usize = 1;

/// Maximum size of thread pool.
const MAX_SIZE: usize = 10000;

/// Env variable that allows override default value of max worker count.
const ENV_SIZE_NAME: &str = "BLOCKING_POOL_SIZE";

/// When waitting task count is bigger than `idle_count * BUSY_TIMES`,
/// pool will try to wake all worker up and spawn a new worker.
const BUSY_TIMES: usize = 4;

/// If there is no task, the worker thread will terminate after `TIMEOUT`ms.
const TIMEOUT: u64 = 500;

/// Singleton of the thread pool.
static GLOBAL: OnceLock<ThreadPool> = OnceLock::new();

/// The blocking executor.
pub struct ThreadPool {
    /// Inner state of thread pool.
    inner: Mutex<Inner>,
    /// Waiting queue of blocking tasks.
    queue: Arc<ConcurrentQueue<OnceTask>>,
    /// Control workers' sleep and wake up.
    notifier: Condvar,
}

/// Inner state of the thread pool.
pub struct Inner {
    /// The number of idle workers.
    idle_count: usize,
    /// Total number of worker threads.
    ///
    /// Equal to idle threads + active threads.
    thread_count: usize,
    /// Maximum number of workers in the pool.
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

    /// Spawns a FnOnce onto this thread pool.
    ///
    /// Return a [`OnceTaskHandle`] for the spawned task.
    fn spawn_blocking<F, T>(&'static self, f: F) -> OnceTaskHandle<T>
    where
        F: FnOnce() -> T,
    {
        let (task, handle) = task_and_handle(f);

        debug_assert!(self.queue.push(task.into()).is_ok());

        self.schedule(self.inner.lock().unwrap());

        handle
    }

    /// Start a new worker on the current thread.
    ///
    /// Worker can control self idle and terminate.
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

    /// Wake up workers and spawn more workers as needed.
    fn schedule(&'static self, mut inner: MutexGuard<'static, Inner>) {
        self.notifier.notify_one();

        while self.queue.len() > inner.idle_count * BUSY_TIMES
            && inner.thread_count < inner.pool_size
        {
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
