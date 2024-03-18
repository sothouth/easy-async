use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub enum MaybeDone<F: Future> {
    Future(F),
    Done(F::Output),
    Taken,
}

impl<F: Future> MaybeDone<F> {
    pub fn from_future(future: F) -> Self {
        Self::Future(future)
    }

    pub fn take_output(&mut self) -> Option<F::Output> {
        match *self {
            Self::Done(_) => match std::mem::replace(self, Self::Taken) {
                Self::Done(output) => Some(output),
                _ => unreachable!(),
            },
            _ => None,
        }
    }
}

impl<F: Future> Future for MaybeDone<F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            match *self.as_mut().get_unchecked_mut() {
                Self::Future(ref mut future) => {
                    let output = ready!(Pin::new_unchecked(future).poll(cx));
                    self.set(Self::Done(output));
                }
                Self::Done(_) => (),
                Self::Taken => unreachable!(),
            }
        }
        Poll::Ready(())
    }
}
