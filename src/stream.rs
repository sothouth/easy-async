use std::{
    async_iter::AsyncIterator,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct NextFuture<'a, S: Unpin + ?Sized> {
    stream: &'a mut S,
}

impl<S: AsyncIterator + Unpin + ?Sized> Future for NextFuture<'_, S> {
    type Output = Option<S::Item>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}

impl<T: AsyncIterator + ?Sized> AsyncIteratorExt for T {}

pub trait AsyncIteratorExt: AsyncIterator {
    fn next(&mut self) -> NextFuture<Self>
    where
        Self: Unpin,
    {
        NextFuture { stream: self }
    }
}
