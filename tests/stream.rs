#![feature(async_iterator)]

use std::{
    async_iter::AsyncIterator,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use easy_async::stream::AsyncIteratorExt;

#[async_std::test]
async fn main() {
    let mut cur = 0;
    let mut counter = Counter::new();
    while let Some(i) = counter.next().await {
        println!("{}", i);
        assert_eq!(i, cur);
        cur += 1;
    }
    assert_eq!(cur, 6);
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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let res = self.cur;
        self.cur += 1;
        if res < 6 {
            Poll::Ready(Some(res))
        } else {
            Poll::Ready(None)
        }
    }
}