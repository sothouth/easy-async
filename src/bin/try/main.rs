#![feature(async_iterator)]
#![feature(future_join)]
#![feature(gen_future)]
use std::async_iter::{self, IntoAsyncIterator};
use std::future::{self, join, Future};
use std::pin;
use std::process;
use std::sync;
use std::task::{self,Poll};
use std::thread;

use futures::future::MaybeDone;
mod refer{
    use tokio;
    use smol;
    use futures;
    use async_std;
    use futures_timer;
}

struct Counter{
    count:usize,
}
impl Counter{
    fn new()->Self{
        Self{count:0}
    }
}

impl async_iter::AsyncIterator for Counter{
    type Item=usize;

    fn poll_next(mut self: pin::Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Option<Self::Item>> {
        self.count+=1;
        if self.count<6{
            Poll::Ready(Some(self.count))
        }
        else{
            Poll::Ready(None)
        }

    }
}



fn main(){
    let a=async{1};
    easy_async::block_on::block_on(a);
    join!(async{1},async{2});

}

