

use std::task::{self,Waker,Wake,Poll};
use std::future::Future;
use std::pin::Pin;

struct MyPrimitiveFuture{
    state:i32,
    waker:Option<Waker>,
}


impl Future for MyPrimitiveFuture{
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        if self.state==3{
            return Poll::Ready(());
        }
        self.waker=Some(cx.waker().clone());
        Poll::Pending
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn try_my_primitive_future() {
        use async_std::task;
        use async_std::prelude::*;
        task::block_on();
    }
}
