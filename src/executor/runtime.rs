use std::sync::{Arc, Mutex, RwLock};
use std::task::Waker;

use concurrent_queue::ConcurrentQueue;
use slab::Slab;

use async_task::Runnable;

/// runtime core
pub struct Runtime {
    pub(super) global_queue: ConcurrentQueue<Runnable>,
    pub(super) local_queues: RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>,
    pub(super) tasks: Mutex<Slab<Waker>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_queue: ConcurrentQueue::unbounded(),
            local_queues: RwLock::new(Vec::new()),
            tasks: Mutex::new(Slab::new()),
        }
    }
}
