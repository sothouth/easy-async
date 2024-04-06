use std::cell::UnsafeCell;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use async_task::{Builder as TaskBuilder, Runnable, Task};

use crate::utils::call_on_drop::CallOnDrop;

use super::runtime::Runtime;

/// handle for runtime
pub struct Executor<'a> {
    pub(super) rt: Arc<Runtime>,
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

    pub fn run(&self) {}

    pub fn schedule(&self) -> impl Fn(Runnable) + Send + Sync + 'static {
        let rt = self.rt.clone();

        move |runnable| {
            rt.global_queue.push(runnable).unwrap();
        }
    }
}
