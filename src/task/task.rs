/// tring
use std::future::{poll_fn, Future};
use std::pin::{pin, Pin};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering::*};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::{Duration, Instant};

use concurrent_queue::ConcurrentQueue;
use num_cpus;

use async_task::{Builder as TaskBuilder, Runnable, Schedule, Task as _};

mod refer {
    use async_executor;
    use async_task::{Runnable, Task as _};
    use smol;
    use tokio::runtime::Builder;
    use tokio::runtime::Handle;
}

// pub struct Task<T> {
//     exe: Arc<Todo<T>>,
// }

// pub struct RawTask{

// }



// pub struct Task<T> {
//     state: AtomicUsize,
//     future: F,
//     waker: W,
// }

// pub struct Todo {
//     future: Pin<Box<dyn Future<Output=Any> + Send>>,
//     output: Option<NonNull<()>>,
// }


// impl Todo{
//     fn build<F,S>(future:F,schedule:S)->Todo{

//     }
// }