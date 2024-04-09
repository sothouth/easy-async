use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

use super::Queue;

const EMPTY: usize = 1 << 0;
const FULL: usize = 1 << 1;
const LOCKED: usize = 1 << 2;

pub struct Single<T> {
    state: AtomicUsize,
    slot: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T: Send> Send for Single<T> {}
unsafe impl<T: Send> Sync for Single<T> {}

impl<T> Single<T> {
    pub const fn new() -> Self {
        Self {
            state: AtomicUsize::new(EMPTY),
            slot: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }
}

impl<T> Queue<T> for Single<T> {
    fn push(&self, value: T) -> Result<(), T> {
        match self
            .state
            .compare_exchange(EMPTY, LOCKED | FULL, AcqRel, Acquire)
        {
            Ok(_) => {
                unsafe { (*self.slot.get()).write(value) };
                self.state.fetch_and(!LOCKED, Release);
                Ok(())
            }
            Err(_) => Err(value),
        }
    }

    fn pop(&self) -> Result<T, ()> {
        match self
            .state
            .compare_exchange(FULL, LOCKED | EMPTY, AcqRel, Acquire)
        {
            Ok(_) => {
                let value = unsafe { (*self.slot.get()).assume_init_read() };
                self.state.fetch_and(!LOCKED, Release);
                Ok(value)
            }
            Err(_) => Err(()),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        usize::from(self.state.load(Acquire) & FULL != 0)
    }

    #[inline]
    fn capacity(&self) -> usize {
        1
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn is_full(&self) -> bool {
        self.len() == 1
    }

    #[inline]
    fn slack(&self) -> usize {
        usize::from(self.state.load(Acquire) & EMPTY != 0)
    }
}

impl<T> Drop for Single<T> {
    fn drop(&mut self) {
        if self.len() == 1 {
            unsafe { (*self.slot.get()).assume_init_drop() };
        }
    }
}
