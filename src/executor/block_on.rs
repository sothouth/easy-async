use crate::task::parker_and_waker::parker_and_waker;
use std::{
    future::Future,
    pin,
    task::{Context, Poll},
};

pub fn block_on<F: Future>(fut: F) -> F::Output {
    let (parker, waker) = parker_and_waker();
    let mut cx = Context::from_waker(&waker);
    let mut fut = pin::pin!(fut);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Pending => parker.park(),
            Poll::Ready(res) => return res,
        }
    }
}
