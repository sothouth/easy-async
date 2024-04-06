/// imitate smol
use std::cell::UnsafeCell;
use std::future::{poll_fn, Future};
use std::marker::PhantomData;
use std::pin::{pin, Pin};
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::{Duration, Instant};

use concurrent_queue::ConcurrentQueue;
use num_cpus;
use slab::Slab;

use async_task::{Builder as TaskBuilder, Runnable, Task};

use crate::utils::call_on_drop::CallOnDrop;

mod refer {
    use async_executor;
    use async_task::{Runnable, Task};
    use smol;
    use tokio::runtime::Builder;
    // use monoio;
}


const LOCAL_QUEUE_SIZE: usize = 512;

/// runtime core
pub struct Runtime {
    global_queue: ConcurrentQueue<Runnable>,
    local_queues: RwLock<Vec<Arc<ConcurrentQueue<Runnable>>>>,
    tasks: Mutex<Slab<Waker>>,
}

/// handle for runtime
pub struct Executor<'a> {
    rt: Arc<Runtime>,
    _marker: PhantomData<UnsafeCell<&'a ()>>,
}

unsafe impl Send for Executor<'_> {}
unsafe impl Sync for Executor<'_> {}

/// single thread worker
pub struct Worker<'a> {
    rt: &'a Runtime,
    queue: Arc<ConcurrentQueue<Runnable>>,
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

#[cfg(test)]
mod try_some {
    #[test]
    fn t() {
        use async_executor::Executor;
        use futures_lite::future;

        let ex = Executor::new();

        let t2 = ex.spawn(async {
            println!("t2");
            "hello"
        });
        let task = ex.spawn(async {
            println!("t1");
            1 + 2
        });
        let res = future::block_on(ex.run(async {
            println!("t3");
            task.await * 2
        }));

        assert_eq!(res, 6);
    }

    #[test]
    fn multi_executor() {
        use async_channel::unbounded;
        use async_executor::Executor;
        use easy_parallel::Parallel;
        use futures_lite::future;

        let ex = Executor::new();
        let (signal, shutdown) = unbounded::<()>();

        Parallel::new()
            // Run four executor threads.
            .each(0..4, |_| future::block_on(ex.run(shutdown.recv())))
            // Run the main future on the current thread.
            .finish(|| {
                future::block_on(async {
                    println!("Hello world!");
                    drop(signal);
                })
            });
    }

    #[test]
    fn tcp_server() {
        use std::net::{TcpListener, TcpStream};

        use smol::{io, Async};

        /// Echoes messages from the client back to it.
        async fn echo(stream: Async<TcpStream>) -> io::Result<()> {
            io::copy(&stream, &mut &stream).await?;
            Ok(())
        }

        fn main() -> io::Result<()> {
            smol::block_on(async {
                // Create a listener.
                let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 7000))?;
                println!("Listening on {}", listener.get_ref().local_addr()?);
                println!("Now start a TCP client.");

                // Accept clients in a loop.
                loop {
                    let (stream, peer_addr) = listener.accept().await?;
                    println!("Accepted client: {}", peer_addr);

                    // Spawn a task that echoes messages from the client back to it.
                    smol::spawn(echo(stream)).detach();
                }
            })
        }
        let _ = main();
    }
}
