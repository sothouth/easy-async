
use std::sync::Arc;
use std::task::Waker;
use std::sync::Mutex;
use std::thread;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;


pub fn spawn_blocking<F,T>(clouse:F)->SpawnBlocking<T>
where
    F:FnOnce()->T,
    F:Send+'static,
    T:Send+'static,
{
    let inner=Arc::new(Mutex::new(Shared{
        value:None,
        waker:None,
    }));

    thread::spawn({
        let inner=inner.clone();
        move||{
            let value=clouse();

            let waker={
                let mut guard=inner.lock().unwrap();
                guard.value=Some(value);
                guard.waker.take()
            };

            if let Some(waker)=waker{
                waker.wake();
            }
        }
    });
    SpawnBlocking(inner)
}


pub struct SpawnBlocking<T>(Arc<Mutex<Shared<T>>>);

struct Shared<T>{
    value:Option<T>,
    waker:Option<Waker>,
}

impl<T> Future for SpawnBlocking<T>{
    type Output=T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard=self.0.lock().unwrap();
        if let Some(value)=guard.value.take(){
            Poll::Ready(value)
        }
        else{
            guard.waker=Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
