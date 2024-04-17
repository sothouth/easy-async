use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub fn poll_once<'a, F>(f: Pin<&'a mut F>) -> PollOnce<'a, F> {
    PollOnce { f }
}

pub struct PollOnce<'a, F> {
    f: Pin<&'a mut F>,
}

impl<F, T> Future for PollOnce<'_, F>
where
    F: Future<Output = T>,
{
    type Output = Option<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.f.as_mut().poll(cx) {
            Poll::Ready(t) => Poll::Ready(Some(t)),
            Poll::Pending => Poll::Ready(None),
        }
    }
}
