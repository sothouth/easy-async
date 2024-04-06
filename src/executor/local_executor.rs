/// tring
use std::future::{poll_fn, Future};
use std::pin::{pin, Pin};
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::{Duration, Instant};

use concurrent_queue::ConcurrentQueue;
use num_cpus;

use async_task::{Builder as TaskBuilder, Runnable, Task};

mod refer {
    use async_executor;
    use async_task::{Runnable, Task};
    use smol;
    use tokio::runtime::Builder;
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn spawn<T>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    todo!()
}

const LOCAL_QUEUE_SIZE: usize = 512;
const GLOBAL_QUEUE_SIZE: usize = 0;

pub struct Builder {
    executor_num: usize,
    local_queue_size: usize,
    global_queue_size: usize,
}

pub struct Runtime<'a> {
    global_queue: ConcurrentQueue<Runnable>,
    local_queues: Vec<&'a ConcurrentQueue<Runnable>>,
}

pub struct Excutor {
    queue: ConcurrentQueue<Runnable>,
}

// pub struct LocalExecutor {}



impl Builder {
    pub fn new() -> Self {
        Self {
            executor_num: num_cpus::get(),
            local_queue_size: LOCAL_QUEUE_SIZE,
            global_queue_size: GLOBAL_QUEUE_SIZE,
        }
    }

    pub fn build(self) -> Runtime<'static> {
        todo!()
    }

    pub fn worker_threads(mut self, num: usize) -> Self {
        self.executor_num = num;
        self
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
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
