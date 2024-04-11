use std::cell::UnsafeCell;
use std::fmt;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

pub const NONE: usize = 1 << 0;
pub const SOME: usize = 1 << 1;
pub const LOCK: usize = 1 << 2;

pub struct Slot<T> {
    pub state: AtomicUsize,
    pub value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for Slot<T> {}
unsafe impl<T> Sync for Slot<T> {}

impl<T> Slot<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            state: AtomicUsize::new(NONE),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    pub fn set(&self, value: T) -> Result<(), T> {
        unsafe {
            match self.try_lock_none() {
                Ok(_) => {
                    self.unchecked_set(value);
                    Ok(())
                }
                Err(_) => Err(value),
            }
        }
    }

    #[inline]
    pub unsafe fn try_lock_none(&self) -> Result<usize, usize> {
        self.state
            .compare_exchange(NONE, LOCK | SOME, AcqRel, Acquire)
    }

    #[inline]
    pub unsafe fn unchecked_set(&self, value: T) {
        (*self.value.get()).write(value);
        self.state.store(SOME, Release);
    }

    #[inline]
    pub fn get(&self) -> Result<T, ()> {
        unsafe {
            match self.try_lock_some() {
                Ok(_) => Ok(self.unchecked_get()),
                Err(_) => Err(()),
            }
        }
    }

    #[inline]
    pub unsafe fn try_lock_some(&self) -> Result<usize, usize> {
        self.state
            .compare_exchange(SOME, LOCK | NONE, AcqRel, Acquire)
    }

    /// Get value without check
    #[inline]
    pub unsafe fn unchecked_get(&self) -> T {
        let value = (*self.value.get()).assume_init_read();
        self.state.store(NONE, Release);
        value
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.state.load(Acquire) == NONE
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        self.state.load(Acquire) == SOME
    }
}

impl<T> Default for Slot<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Slot<T> {
    fn drop(&mut self) {
        if self.state.load(Acquire) == SOME {
            unsafe { self.value.get_mut().assume_init_drop() }
        }
    }
}

impl<T> fmt::Debug for Slot<T> {
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
