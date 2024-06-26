//! A simple and easy to used asynchronous runtime.
//!
//! The runtime includes two main components:
//! * Asynchronous Executor
//! * Blocking Executor

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
#![feature(alloc_layout_extra)]
#![feature(const_alloc_layout)]

pub mod prelude;

mod block_on;
pub use block_on::block_on;
pub use block_on::block_on_naive;

mod task;
pub use task::OnceTaskHandle;
pub use task::TaskHandle;

pub mod executor;
pub use executor::spawn;

pub mod blocking;
pub use blocking::spawn_blocking;

#[cfg(feature = "unblock")]
pub mod unblock;
#[cfg(feature = "unblock")]
pub use unblock::Unblock;

pub mod stream;

pub mod waker;

pub mod future;

pub mod utils;
