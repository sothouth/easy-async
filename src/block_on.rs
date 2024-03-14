use waker_fn::waker_fn;
use crossbeam::sync::Parker;
// use std::thread::park;
use std::future::Future;
// use futures_lite::pin;
use std::pin::pin;
use std::task::{Context, Poll};

pub fn block_on<F: Future>(future: F) -> F::Output {
    let parker = Parker::new();
    let unparker = parker.unparker().clone();
    let waker = waker_fn(move || unparker.unpark());
    let mut cx = Context::from_waker(&waker);

    let mut pinned_fut = pin!(future);
    loop {
        match pinned_fut.as_mut().poll(&mut cx) {
            Poll::Pending => parker.park(),
            Poll::Ready(res) => return res,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn try_block_on() {
        use crate::block_on::block_on;
        use crate::spawn_blocking::spawn_blocking;
        assert_eq!(block_on(spawn_blocking(|| { 1 })), 1);
    }
}
