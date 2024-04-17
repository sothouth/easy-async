/// imitate smol
use std::cell::UnsafeCell;
use std::future::{poll_fn, Future};
use std::marker::PhantomData;
use std::pin::{pin, Pin};
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;

use concurrent_queue::ConcurrentQueue;
use slab::Slab;

use async_task::{Builder as TaskBuilder, Runnable, Task};

use crate::utils::CallOnDrop;

mod refer {
    use async_executor;
    use async_task::{Runnable, Task};
    use smol;
    use tokio::runtime::Builder;
    // use monoio;
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
