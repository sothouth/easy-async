use std::cell::UnsafeCell;
use std::fmt;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

const NONE: usize = 1 << 0;
const SOME: usize = 1 << 1;
const LOCK: usize = 1 << 2;

pub struct Slab<T> {
    state: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
}

impl<T> Slab<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(NONE),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    pub fn set(&self, value: T) -> Result<(), T> {
        match self
            .state
            .compare_exchange(NONE, LOCK | SOME, AcqRel, Acquire)
        {
            Ok(_) => {
                unsafe {
                    (*self.value.get()).write(value);
                }
                self.state.store(SOME, Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }

    #[inline]
    pub fn get(&self) -> Result<T, ()> {
        match self
            .state
            .compare_exchange(SOME, LOCK | NONE, AcqRel, Acquire)
        {
            Ok(_) => {
                let value = unsafe { (*self.value.get()).assume_init_read() };
                self.state.store(NONE, Release);
                Ok(value)
            }
            Err(_) => Err(()),
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.state.load(Acquire) == NONE
    }
}

impl<T> Default for Slab<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for Slab<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slab")
            .field(
                "state",
                &match self.state.load(Acquire) {
                    NONE => "NONE",
                    SOME => "SOME",
                    state if state == LOCK | NONE => "LOCK|NONE",
                    state if state == LOCK | SOME => "LOCK|SOME",
                    _ => unreachable!("invalid state"),
                },
            )
            .field("value", &"..")
            .finish()
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        if self.state.load(Acquire) == SOME {
            unsafe { self.value.get_mut().assume_init_drop() }
        }
    }
}
