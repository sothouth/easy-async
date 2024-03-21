use std::{
    cell::UnsafeCell,
    fmt,
    sync::atomic::{AtomicUsize, Ordering::*},
    task::Waker,
};

const WAITING: usize = 0b00;
const WAKING: usize = 0b01;
const REGISTERING: usize = 0b10;

pub struct AtomicWaker {
    state: AtomicUsize,
    waker: UnsafeCell<Option<Waker>>,
}

unsafe impl Send for AtomicWaker {}
unsafe impl Sync for AtomicWaker {}

impl AtomicWaker {
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(WAITING),
            waker: UnsafeCell::new(None),
        }
    }

    pub fn wake(&self) {
        if let Some(waker) = self.take() {
            waker.wake();
        }
    }

    pub fn take(&self) -> Option<Waker> {
        match self.state.fetch_or(WAKING, AcqRel) {
            WAITING => {
                let waker = unsafe { (*self.waker.get()).take() };
                self.state.fetch_and(!WAKING, Release);
                waker
            }
            state => {
                debug_assert!(
                    state == REGISTERING || state == REGISTERING | WAKING || state == WAKING
                );
                None
            }
        }
    }

    pub fn register(&self, waker: &Waker) {
        match self
            .state
            .compare_exchange(WAITING, REGISTERING, AcqRel, Acquire)
        {
            Ok(_) => {
                unsafe {
                    if let Some(ref cur_waker) = *self.waker.get() {
                        if !cur_waker.will_wake(waker) {
                            *self.waker.get() = Some(waker.clone());
                        }
                    } else {
                        *self.waker.get() = Some(waker.clone());
                    }
                }
                let res = self
                    .state
                    .compare_exchange(REGISTERING, WAITING, AcqRel, Acquire);
                if let Err(state) = res {
                    debug_assert_eq!(state, REGISTERING | WAKING);
                    let waker = unsafe { (*self.waker.get()).take() }.unwrap();
                    // cannot use self.state.store(WAITING, Release);
                    // because waker might be woken by other thread
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
