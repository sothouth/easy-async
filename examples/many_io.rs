#![feature(thread_sleep_until)]
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    const N: usize = 10;
    const TIME: usize = 1000;

    let mut handles = Vec::with_capacity(N);

    for i in 1..=N {
        handles.push(easy_async::spawn(SimIO::new(
            i,
            Duration::from_millis((TIME / i) as u64),
            i - 1,
        )));
    }

    for handle in handles {
        easy_async::block_on(handle);
    }
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
