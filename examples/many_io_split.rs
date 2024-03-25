#![feature(thread_sleep_until)]
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;
use std::time::Instant;

use async_std::prelude::FutureExt;
use futures::future::join_all;
use tokio::runtime::{Builder, Runtime};
use tokio::time::sleep;

fn main() {
    let num = 10;
    let sum_time = 1000;
    let rt = Builder::new_current_thread().build().unwrap();

    let mut handles = Vec::with_capacity(num);

    for i in 1..=num {
        handles.push(rt.spawn(SimIO::new(
            i,
            Duration::from_millis((sum_time / i) as u64),
            i - 1,
        )));
    }

    rt.block_on(join_all(handles));
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
