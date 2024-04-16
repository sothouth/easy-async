use std::collections::VecDeque;
use std::env;
use std::future::Future;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

// use crate::task::task::{task_and_handle, TaskHandle};

mod task;

static GLOBAL: OnceLock<ThreadPool> = OnceLock::new();

pub fn spawn_blocking(f: impl FnOnce() + Send + 'static) {
    GLOBAL.get_or_init(|| ThreadPool::new()).spawn_blocking(f);
}

const DEFAULT_SIZE: usize = 500;

const MIN_SIZE: usize = 1;

const MAX_SIZE: usize = 10000;

const ENV_SIZE_NAME: &str = "BLOCKING_POOL_SIZE";

pub struct ThreadPool {
    inner: Mutex<Inner>,
    notifier: Condvar,
}

pub struct Inner {
    idle_count: usize,
    thread_count: usize,
    queue: VecDeque<Runnable>,
    pool_size: usize,
}

impl Inner {
    fn new(pool_size: usize) -> Self {
        Self {
            idle_count: 0,
            thread_count: 0,
            pool_size,
            queue: VecDeque::new(),
        }
    }
}

impl ThreadPool {
    fn new() -> Self {
        let pool_size = match env::var(ENV_SIZE_NAME) {
            Ok(s) => s.parse().unwrap_or(DEFAULT_SIZE),
            Err(_) => DEFAULT_SIZE,
        };

        assert!(pool_size >= MIN_SIZE);
        assert!(pool_size <= MAX_SIZE);

        Self {
            inner: Mutex::new(Inner::new(pool_size)),
            notifier: Condvar::new(),
        }
    }

    fn spawn_blocking(&self, f: impl FnOnce() + Send + 'static) {
        // let (task, handle) = task_and_handle();
    }
}
