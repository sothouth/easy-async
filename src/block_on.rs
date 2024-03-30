use crate::task::waker::current_thread_waker;
use std::{
    future::Future,
    pin,
    task::{Context, Poll},
    thread,
};

pub fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = current_thread_waker();
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Pending => thread::park(),
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

mod old {
    use crossbeam::sync::Parker;
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll};
    use waker_fn::waker_fn;

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
}
