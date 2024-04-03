#![feature(decl_macro)]
#![feature(async_iterator)]
#![feature(future_join)]
#![feature(noop_waker)]
#![feature(waker_getters)]
#![feature(inline_const)]
#![feature(const_trait_impl)]
#![feature(effects)]

mod temp;
mod timer_future;

pub mod executor_task;

pub mod prelude;
pub use executor::block_on::block_on;

pub mod atomic_waker;
pub mod future;
pub mod macros;
pub mod stream;
pub mod task;

pub mod executor;
pub mod spawn_blocking;

mod refer;

pub mod utils;
