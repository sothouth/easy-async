#![feature(async_iterator)]

use std::async_iter::AsyncIterator;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

fn main() {
    easy_async::block_on(async {
        let mut cur = 0;
        let mut counter = Counter::new();
        while let Some(i) = counter.next().await {
            println!("{}", i);
            assert_eq!(i, cur);
            cur += 1;
        }
        assert_eq!(cur, 6);
    });
}

struct Counter {
    cur: usize,
}

impl Counter {
    fn new() -> Self {
        Self { cur: 0 }
    }
}

impl AsyncIterator for Counter {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let res = self.cur;
        self.cur += 1;
        if res < 6 {
            Poll::Ready(Some(res))
        } else {
            Poll::Ready(None)
        }
    }
}

struct Stream<'a, T: Unpin + ?Sized> {
    stream: &'a mut T,
}

trait AsyncIteratorExt: AsyncIterator {
    fn next(&mut self) -> Stream<Self>
    where
        Self: Unpin,
    {
        Stream { stream: self }
    }
}

impl<T: AsyncIterator> AsyncIteratorExt for T {}

impl<T: AsyncIterator + Unpin + ?Sized> Future for Stream<'_, T> {
    type Output = Option<T::Item>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}
