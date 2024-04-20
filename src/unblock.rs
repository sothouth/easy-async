// use crate::future::poll_fn;

// pub struct Unblock<T> {
//     state: State<T>,
//     cap: Option<usize>,
// }

// impl<T> Unblock<T> {
//     pub fn new(io: T) -> Self {
//         Self {
//             state: State::Idle(Some(Box::new(io))),
//             cap: None,
//         }
//     }

//     pub fn with_capacity(cap: usize, io: T) -> Self {
//         Self {
//             state: State::Idle(Some(Box::new(io))),
//             cap: Some(cap),
//         }
//     }

//     pub async fn get_mut(&mut self) -> &mut T {
//         poll_fn(|cx| self.poll_stop(cx)).await.ok();

//         match &mut self.state {}
//     }
// }

// enum State<T> {
//     Idle(Option<Box<T>>),
//     WithMut(TaskHandle<Box<T>>),
//     Streaming(Option<Box<dyn Any +Send+Sync>>,)
// }
