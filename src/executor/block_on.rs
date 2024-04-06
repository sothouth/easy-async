use crate::waker::{parker_and_waker, Parker};
use std::cell::RefCell;
use std::{
    future::Future,
    pin,
    task::{Context, Poll, Waker},
};

pub fn block_on<F: Future>(fut: F) -> F::Output {
    let mut fut = pin::pin!(fut);

    thread_local! {
        static CACHE:RefCell<(Parker, Waker)> = RefCell::new(parker_and_waker());
    }

    CACHE.with(|cache| {
        let cached;
        let fresh;
        let (parker, waker) = match cache.try_borrow_mut() {
            Ok(cache) => {
                cached = cache;
                &*cached
            }
            Err(_) => {
                fresh = parker_and_waker();
                &fresh
            }
        };

        let cx = &mut Context::from_waker(&waker);

        loop {
            match fut.as_mut().poll(cx) {
                Poll::Pending => parker.park(),
                Poll::Ready(output) => return output,
            }
        }
    })
}

pub fn block_on_naive<F: Future>(fut: F) -> F::Output {
    let mut fut = pin::pin!(fut);
    let (parker, waker) = parker_and_waker();
    let cx = &mut Context::from_waker(&waker);
    loop {
        match fut.as_mut().poll(cx) {
            Poll::Pending => parker.park(),
            Poll::Ready(output) => return output,
        }
    }
}
