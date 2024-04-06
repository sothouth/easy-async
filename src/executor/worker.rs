use std::sync::Arc;

use concurrent_queue::ConcurrentQueue;

use async_task::Runnable;

use super::runtime::Runtime;

const LOCAL_QUEUE_SIZE: usize = 512;

/// single thread worker
pub struct Worker<'a> {
    rt: &'a Runtime,
    queue: Arc<ConcurrentQueue<Runnable>>,
}

impl<'a> Worker<'a> {
    pub fn new(rt: &'a Runtime) -> Self {
        let queue = Arc::new(ConcurrentQueue::bounded(LOCAL_QUEUE_SIZE));

        rt.local_queues.write().unwrap().push(queue.clone());

        Self { rt, queue }
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
        todo!()
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
