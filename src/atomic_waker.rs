use std::{
    cell::UnsafeCell,
    fmt, mem,
    sync::atomic::{AtomicUsize, Ordering::*},
    task::Waker,
};

const WAITING: usize = 0b00;
const WAKING: usize = 0b01;
const REGISTERING: usize = 0b10;

pub struct AtomicWaker {
    state: AtomicUsize,
    waker: UnsafeCell<Waker>,
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

impl AtomicWaker {
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(WAITING),
            waker: UnsafeCell::new(Waker::noop().clone()),
        }
    }

    #[inline]
    pub fn wake(&self) {
        // don't need to check waker, because NOOP will do nothing
        self.take().wake();
    }

    #[inline]
    fn replace(&self, waker: Waker) -> Waker {
        mem::replace(unsafe { &mut *self.waker.get() }, waker)
    }

    pub fn take(&self) -> Waker {
        match self.state.fetch_or(WAKING, AcqRel) {
            WAITING => {
                let waker = self.replace(Waker::noop().clone());
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

    pub fn register(&self, waker: &Waker) {
        match self
            .state
            .compare_exchange(WAITING, REGISTERING, AcqRel, Acquire)
        {
            Ok(_) => {
                // check and clone maybe slightly faster
                unsafe { (*self.waker.get()).clone_from(waker) };

                let res = self
                    .state
                    .compare_exchange(REGISTERING, WAITING, AcqRel, Acquire);
                if let Err(actual) = res {
                    debug_assert_eq!(actual, REGISTERING | WAKING);
                    let waker = self.replace(Waker::noop().clone());
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
