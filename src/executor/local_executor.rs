use std::future::{poll_fn, Future};
use std::pin::{pin, Pin};
/// tring
use std::sync::{Arc, Mutex, RwLock};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::thread;
use std::time::{Duration, Instant};

pub struct Builder {}

pub struct Excutor {}

pub struct LocalExecutor {}

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
}

#[test]
fn t() {
    use async_executor::Executor;
    use futures_lite::future;

    let ex = Executor::new();

    let task = ex.spawn(async { 1 + 2 });
    let t2 = ex.spawn(async { "hello" });
    let res = future::block_on(ex.run(async { task.await * 2 }));

    assert_eq!(res, 6);
}
