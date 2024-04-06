use std::future::Future;
use std::sync::OnceLock;
use std::thread;

use super::local_executor::Executor;

use async_task::Task;

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

                        crate::block_on(async { ex.run() });
                    })
                    .expect("spawn worker thread error");
            }
            ex
        })
        .spawn(future)
}
