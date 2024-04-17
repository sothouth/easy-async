#![feature(decl_macro)]
#![feature(async_iterator)]
#![feature(future_join)]
#![feature(noop_waker)]
#![feature(waker_getters)]
#![feature(inline_const)]
#![feature(const_trait_impl)]
#![feature(effects)]
#![feature(const_waker)]
#![feature(future_poll_fn)]
// #![feature(unboxed_closures)]
// #![feature(fn_traits)]

mod timer_future;

pub mod executor_task;

pub mod prelude;

pub mod executor;
pub use executor::block_on::block_on;

pub mod blocking;
// pub use

pub mod waker;

pub mod future;
pub mod macros;
pub mod stream;
pub mod task;

pub mod spawn_blocking;

mod refer;

pub mod utils;
