#![feature(decl_macro)]
#![feature(async_iterator)]
#![feature(future_join)]
#![feature(noop_waker)]
#![feature(waker_getters)]
#![feature(inline_const)]
#![feature(const_trait_impl)]
#![feature(effects)]
#![feature(const_waker)]
// #![feature(unboxed_closures)]
// #![feature(fn_traits)]
// #![feature(future_poll_fn)]

mod timer_future;

pub mod executor_task;

pub mod prelude;

mod block_on;
pub use block_on::block_on;
pub use block_on::block_on_naive;

pub mod executor;

pub mod blocking;
pub use blocking::spawn_blocking;

pub mod waker;

pub mod future;
pub mod macros;
pub mod stream;
pub mod task;

mod refer;

pub mod utils;
