
pub mod thread_waker;
pub mod empty_waker;
pub mod parker_and_waker;
pub mod option_waker;


use std::future::Future;
use std::sync::{Arc,atomic::{AtomicUsize,Ordering::*}};
use std::task::{Context,Waker,Poll};

pub struct Task<F,W>{
    state:AtomicUsize,
    future:F,
    waker:W,
}
