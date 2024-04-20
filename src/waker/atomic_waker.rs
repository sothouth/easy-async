use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering::*};
use std::task::Waker;

use super::OptionWaker;

/// Idle state
const WAITING: usize = 0b00;

/// The waker currently registered with the [`AtomicWaker`] cell is being woken.
const WAKING: usize = 0b01;

/// A new waker value is being registered with the [`AtomicWaker`] cell.
const REGISTERING: usize = 0b10;

/// A thread-safe waker that can be shared between threads.
///
/// [`AtomicWaker`] wraps a  [`OptionWaker`] and provides atomic operations
/// for its registration and waking.
pub struct AtomicWaker {
    state: AtomicUsize,
    waker: OptionWaker,
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

impl AtomicWaker {
    /// Create an empty [`AtomicWaker`].
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(WAITING),
            waker: OptionWaker::new(),
        }
    }

    /// Calls `wake` on the last [`Waker`] passed to `register`.
    ///
    /// If `register` has not been called yet, then this does nothing.
    #[inline]
    pub fn wake(&self) {
        // don't need to check waker, because NOOP will do nothing
        self.take().wake();
    }

    /// Returns the last [`Waker`] passed to `register`, so that the user can wake it.
    pub fn take(&self) -> Waker {
        match self.state.fetch_or(WAKING, AcqRel) {
            WAITING => {
                let waker = self.waker.take();
                self.state.fetch_and(!WAKING, Release);
                waker
            }
            state => {
                debug_assert!(
                    state == REGISTERING || state == REGISTERING | WAKING || state == WAKING
                );
                Waker::noop().clone()
            }
        }
    }

    /// Registers the waker to be notified on calls to `wake`.
    pub fn register(&self, waker: &Waker) {
        match self
            .state
            .compare_exchange(WAITING, REGISTERING, AcqRel, Acquire)
        {
            Ok(_) => {
                self.waker.register(waker);

                let res = self
                    .state
                    .compare_exchange(REGISTERING, WAITING, AcqRel, Acquire);
                if let Err(actual) = res {
                    debug_assert_eq!(actual, REGISTERING | WAKING);
                    let waker = self.waker.take();
                    // cannot use self.state.store(WAITING, Release);
                    // because waker might be take by other thread
                    self.state.swap(WAITING, AcqRel);
                    waker.wake();
                }
            }
            Err(WAKING) => {
                waker.wake_by_ref();
            }
            Err(state) => {
                debug_assert!(state == REGISTERING || state == REGISTERING | WAKING);
            }
        }
    }
}

impl Default for AtomicWaker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AtomicWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AtomicWaker")
    }
}
