use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct PendingN {
    print: bool,
    cur: usize,
    num: usize,
}

impl PendingN {
    pub fn new(num: usize, print: bool) -> Self {
        Self { print, cur: 0, num }
    }
}

impl Future for PendingN {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.print {
            println!("PendingN: {}->{}", self.cur, self.num);
        }
        if self.cur == self.num {
            Poll::Ready(())
        } else {
            if self.cur & 1 == 0 {
                cx.waker().clone().wake();
            } else {
                cx.waker().wake_by_ref();
            }
            self.cur += 1;
            Poll::Pending
        }
    }
}
