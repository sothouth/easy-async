use std::cell::RefCell;
use std::future::Future;
use std::pin;
use std::task::{Context, Poll, Waker};

use crate::waker::{parker_and_waker, Parker};

/// Blocks the current thread until the given future is complete.
///
/// # Examples
///
/// ```
/// # use std::thread;
/// # use std::time::Duration;
/// use easy_async::block_on;
///
/// let n=block_on(async {
///     thread::sleep(Duration::from_millis(100));
///     println!("hello");
///     1
/// });
/// assert_eq!(n, 1);
/// ```
///
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

/// Blocks the current thread until the given future is complete.
///
/// # Examples
///
/// ```
/// # use std::thread;
/// # use std::time::Duration;
/// use easy_async::block_on_naive;
///
/// let n=block_on_naive(async {
///     thread::sleep(Duration::from_millis(100));
///     println!("hello");
///     1
/// });
/// assert_eq!(n, 1);
/// ```
///
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
