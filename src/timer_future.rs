use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::*},
        Arc, Mutex,
    },
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use crate::waker::AtomicWaker;

pub struct TimerFuture {
    shared_state: Arc<SharedState>,
}

struct SharedState {
    completed: AtomicBool,
    waker: AtomicWaker,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.shared_state.completed.load(Acquire) {
            return Poll::Ready(());
        }
        self.shared_state.waker.register(cx.waker());
        Poll::Pending
    }
}

impl TimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(SharedState {
            completed: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        });

        let timer_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            timer_shared_state.completed.store(true, Release);
            timer_shared_state.waker.wake();
        });

        Self { shared_state }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_future() {
        let future = TimerFuture::new(Duration::from_millis(100));
        assert!(!future.shared_state.completed.load(Acquire));
        thread::sleep(Duration::from_millis(200));
        assert!(future.shared_state.completed.load(Acquire));
    }
}
