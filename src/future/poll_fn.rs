//! like std::future::poll_fn

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Creates a future that wraps a function returning [`Poll`].
pub fn poll_fn<T, F>(f: F) -> PollFn<F>
where
    F: FnMut(&mut Context) -> Poll<T>,
{
    PollFn(f)
}

/// A Future that wraps a function returning [`Poll`].
///
/// This `struct` is created by [`poll_fn()`].
pub struct PollFn<F>(F);

impl<F: Unpin> Unpin for PollFn<F> {}

impl<F> fmt::Debug for PollFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PollFn").finish()
    }
}

impl<T, F> Future for PollFn<F>
where
    F: FnMut(&mut Context) -> Poll<T>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        (unsafe { &mut self.get_unchecked_mut().0 })(cx)
    }
}
