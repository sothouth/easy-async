use std::{
    async_iter::AsyncIterator,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub struct Stream<'a, T: Unpin + ?Sized> {
    stream: &'a mut T,
}

impl<T: AsyncIterator + Unpin + ?Sized> Future for Stream<'_, T> {
    type Output = Option<T::Item>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}

impl<T: AsyncIterator + ?Sized> AsyncIteratorExt for T {}

pub trait AsyncIteratorExt: AsyncIterator {
    fn next(&mut self) -> Stream<Self>
    where
        Self: Unpin,
    {
        Stream { stream: self }
    }
}
