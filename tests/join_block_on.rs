#![feature(thread_sleep_until)]
#![feature(future_join)]
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    thread,
    time::{Duration, Instant},
};

use futures::future::join_all;

#[test]
fn many_io() {
    let num = 10;
    let sum_time = 1000;

    let handles: Vec<SimIO> = (1..=num)
        .map(|i| SimIO::new(i, Duration::from_millis((sum_time / i) as u64), i - 1))
        .collect();

    easy_async::block_on(join_all(handles));
}

#[test]
fn join_two() {
    use std::future::join;
    let a = async { 1 };
    easy_async::block_on(a);
    easy_async::block_on(join!(async { 1 }, async { 2 }));
}

struct SimIO {
    id: usize,
    end: usize,
    cur: usize,
    next_time: Instant,
    need: Duration,
}

impl SimIO {
    fn new(end: usize, need: Duration, id: usize) -> Self {
        Self {
            id,
            end,
            cur: 0,
            next_time: Instant::now(),
            need,
        }
    }
}

impl Future for SimIO {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        if self.cur == self.end {
            return Poll::Ready(());
        }
        if Instant::now() < self.next_time {
            return Poll::Pending;
        }
        self.cur += 1;
        self.next_time = Instant::now() + self.need;
        println!("{}:{}", self.id, self.cur);

        let waker = cx.waker().clone();
        let need = self.next_time.clone();
        thread::spawn(move || {
            thread::sleep_until(need);
            waker.wake();
        });

        Poll::Pending
    }
}
