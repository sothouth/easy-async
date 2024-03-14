use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    state: bool,
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared_state = self.shared_state.lock().unwrap();
        if shared_state.state {
            return Poll::Ready(());
        }
        shared_state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl TimerFuture {
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            state: false,
            waker: None,
        }));

        let timer_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            let mut shared_state = timer_shared_state.lock().unwrap();
            shared_state.state = true;
            if let Some(waker) = shared_state.waker.take() {
                waker.wake();
            }
        });

        Self { shared_state }
    }
}





