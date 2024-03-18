use std::{
    pin::{Pin,pin},
    future::Future, process::Output, sync::{mpsc::{sync_channel,Receiver,SyncSender}, Arc, Mutex}, task::Context, time::Duration
};

use futures::{
    future::{BoxFuture,FutureExt},
    task::{waker_ref,ArcWake},
};



use crate::timer_future::{self,TimerFuture};

struct Executor{
    ready_queue:Receiver<Arc<Task>>,
}

#[derive(Clone)]
struct Spawner{
    task_sender:SyncSender<Arc<Task>>,
}

struct Task{
    future:Mutex<Option<Pin<Box<dyn Future<Output = ()>+Send+'static>>>>,
    task_sender:SyncSender<Arc<Task>>,
}

fn new_executor_and_spawner()->(Executor,Spawner){
    const MAX_QUEUED_TASKS:usize=1_000;
    let (task_sender,ready_queue)=sync_channel(MAX_QUEUED_TASKS);
    (Executor{ready_queue},Spawner{task_sender})
}

impl Spawner{
    fn spawn(&self,future:impl Future<Output=()>+'static+Send){
        let future=Box::pin(future);
        // let future=pin!(Box::new(future));
        let task=Arc::new(Task{
            future:Mutex::new(Some(future)),
            task_sender:self.task_sender.clone(),
        });
        println!("first dispatch the future task to executor.");
        self.task_sender.send(task).expect("too many tasks queued.");
    }
}

impl ArcWake for Task{
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned=arc_self.clone();
        arc_self.task_sender.send(cloned).expect("too many tasks queued");
    }
}

impl Executor{
    fn run(&self){
        let mut count=0;
        while let Ok(task)=self.ready_queue.recv(){
            count+=1;
            println!("received task. {}",count);

            let mut future_slot=task.future.lock().unwrap();
            if let Some(mut future)=future_slot.take(){
                let waker=waker_ref(&task);
                let context=&mut Context::from_waker(&*waker);
                if future.as_mut().poll(context).is_pending(){
                    println!("excutor run the future task, but is not ready, create a future again.");
                    *future_slot=Some(future);
                }
                else{
                    println!("executor run the future task, is ready. the future task is done. ")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use futures::executor;

    use super::*;
    #[test]
    fn test(){
        let (executor,spawner)=new_executor_and_spawner();
        spawner.spawn(async {
            println!("TimerFuture await");
            TimerFuture::new(Duration::new(10,0)).await;
            println!("TimerFuture Done");
    });
        drop(spawner);
        executor.run();
    }
}