#![feature(async_iterator)]
#![feature(future_join)]
#![feature(gen_future)]
use std::async_iter::{self, IntoAsyncIterator};
use std::future::{self, join, Future};
use std::pin;
use std::process;
use std::sync;
use std::task::{self, Poll};
use std::thread;

use futures::future::MaybeDone;
mod refer {
    use async_std;
    use smol;
    use futures_lite;
    use crossbeam;
    use futures;
    use futures_timer;
    // use mini_tokio;
    use tokio;
    use async_std::{
        task::{sleep,block_on},
        future::timeout,
        stream::interval,
        
    };
    use smol::{
        Executor,
        prelude,
        Timer,
        channel,
        block_on as _,
    };
    use crossbeam::{
        channel::{
            Sender,
            Receiver,
        }
    };
    use tokio::{
        spawn,
        runtime::{Runtime, Builder},
        net::{TcpListener, TcpStream},
        time::{sleep as _, Duration},
    };

    use std::{task::{Context},future::{Future}};
    use std::task::ready;
    use std::thread::{park};
    
}

mod base {
    use async_task;
    use futures_lite;
    use futures_core;
    use async_executor;
    use async_channel;
    use crossbeam;
    use futures;
}

struct Counter {
    count: usize,
}
impl Counter {
    fn new() -> Self {
        Self { count: 0 }
    }
}

impl async_iter::AsyncIterator for Counter {
    type Item = usize;

    fn poll_next(
        mut self: pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        self.count += 1;
        if self.count < 6 {
            Poll::Ready(Some(self.count))
        } else {
            Poll::Ready(None)
        }
    }
}

fn main() {
    let a = async { 1 };
    easy_async::block_on(a);
    join!(async { 1 }, async { 2 });
}
