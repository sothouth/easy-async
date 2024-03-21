use tokio;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
}


// #[cfg(test)]
// mod tests{
//     #[test]
//     fn try_cpu(){
//         use num_cpus::*;
//         println!("cpus: {}", get());
//         println!("cpus: {}", get_physical());
//     }
// }





use crossbeam::channel;
use futures::future::BoxFuture;
use futures::task::{self, ArcWake};

use std::{
    cell::RefCell,
    future::Future,
    sync::{Arc, Mutex},
    task::Context,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
        task::{Context, Poll, Waker},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn test_runtime() {
        async fn delay(dur: Duration) {
            struct Delay {
                when: Instant,

                waker: Option<Arc<Mutex<Waker>>>,
            }

            impl Future for Delay {
                type Output = ();

                fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                    if let Some(waker) = &self.waker {
                        let mut waker = waker.lock().unwrap();

                        if !waker.will_wake(cx.waker()) {
                            *waker = cx.waker().clone();
                        }
                    } else {
                        let when = self.when;
                        let waker = Arc::new(Mutex::new(cx.waker().clone()));
                        self.waker = Some(waker.clone());

                        thread::spawn(move || {
                            let now = Instant::now();

                            if now < when {
                                thread::sleep(when - now);
                            }

                            let waker = waker.lock().unwrap();
                            waker.wake_by_ref();
                        });
                    }

                    if Instant::now() >= self.when {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                }
            }

            let future = Delay {
                when: Instant::now() + dur,
                waker: None,
            };

            future.await;
        }

        let mini_tokio = MiniTokio::new();

        mini_tokio.spawn(async {
            spawn(async {
                delay(Duration::from_millis(100)).await;
                println!("world");
            });

            spawn(async {
                println!("hello");
            });

            delay(Duration::from_millis(200)).await;
            std::process::exit(0);
        });

        mini_tokio.run();
    }
}

pub struct MiniTokio {
    scheduled: channel::Receiver<Arc<Task>>,
    sender: channel::Sender<Arc<Task>>,
}

impl MiniTokio {
    pub fn new() -> MiniTokio {
        let (sender, scheduled) = channel::unbounded();

        MiniTokio { scheduled, sender }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        println!("in spawn init");

        Task::spawn(future, &self.sender);
        println!("in spawn end");
    }

    pub fn run(&self) {
        CURRENT.with(|cell| {
            println!("minitokio init");
            *cell.borrow_mut() = Some(self.sender.clone());
            println!("minitokio end");
        });

        while let Ok(task) = self.scheduled.recv() {
            task.poll();
        }
    }
}

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    CURRENT.with(|cell| {
        
        println!("fn spawn init");
        let borrow = cell.borrow();
        let sender = borrow.as_ref().unwrap();
        Task::spawn(future, sender);
        println!("fn spawn end");

    });
}

thread_local! {
    static CURRENT: RefCell<Option<channel::Sender<Arc<Task>>>> ={
        println!("init CURRENT");
        RefCell::new(None)
    }
        ;
}

struct Task {
    future: Mutex<BoxFuture<'static, ()>>,
    executor: channel::Sender<Arc<Task>>,
}

impl Task {
    fn spawn<F>(future: F, sender: &channel::Sender<Arc<Task>>)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        println!("task spawn init");
        let task = Arc::new(Task {
            future: Mutex::new(Box::pin(future)),
            executor: sender.clone(),
        });

        let _ = sender.send(task);
        println!("task spawn end");

    }

    fn poll(self: Arc<Self>) {
        let waker = task::waker(self.clone());
        let mut cx = Context::from_waker(&waker);
        let mut future = self.future.try_lock().unwrap();
        let _ = future.as_mut().poll(&mut cx);
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) { let _ = arc_self.executor.send(arc_self.clone()); }
}
