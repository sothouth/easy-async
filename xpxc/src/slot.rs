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

    /// Assume state is `NONE`
    #[inline]
    pub unsafe fn try_lock_none(&self) -> Result<usize, usize> {
        self.state
            .compare_exchange(NONE, LOCK | SOME, AcqRel, Acquire)
    }

    /// Assume state is `LOCK|SOME`
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

    /// Assume state is `SOME`
    #[inline]
    pub unsafe fn try_lock_some(&self) -> Result<usize, usize> {
        self.state
            .compare_exchange(SOME, LOCK | NONE, AcqRel, Acquire)
    }

    /// Assume state is `LOCK|NONE`
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

pub const STATE_MASK: usize = 0b111;
pub const ID_SHIFT: usize = 3;
pub const ID_MOD: usize = (!0) >> ID_SHIFT;

pub struct IdSlot<T> {
    pub state: AtomicUsize,
    pub value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for IdSlot<T> {}
unsafe impl<T> Sync for IdSlot<T> {}

impl<T> IdSlot<T> {
    #[inline]
    pub fn new(id: usize) -> Self {
        Self {
            state: AtomicUsize::new((id << ID_SHIFT) | NONE),
            value: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    pub fn set(&self, value: T, id: usize) -> Result<(), T> {
        unsafe {
            match self.try_lock_none(id) {
                Ok(_) => {
                    self.unchecked_set(value, id);
                    Ok(())
                }
                Err(_) => Err(value),
            }
        }
    }

    /// Assume state is `(id << ID_SHIFT) | NONE`
    #[inline]
    pub unsafe fn try_lock_none(&self, id: usize) -> Result<usize, usize> {
        self.state.compare_exchange(
            (id << ID_SHIFT) | NONE,
            (id << ID_SHIFT) | LOCK | SOME,
            AcqRel,
            Acquire,
        )
    }

    /// Assume state is `(old_id << ID_SHIFT) | LOCK | SOME`
    ///
    /// Update state to `(id << ID_SHIFT) | SOME`
    #[inline]
    pub unsafe fn unchecked_set(&self, value: T, id: usize) {
        (*self.value.get()).write(value);
        self.state.store((id << ID_SHIFT) | SOME, Release);
    }

    #[inline]
    pub fn get(&self, old_id: usize, new_id: usize) -> Result<T, ()> {
        unsafe {
            match self.try_lock_some(old_id, new_id) {
                Ok(_) => Ok(self.unchecked_get(new_id)),
                Err(_) => Err(()),
            }
        }
    }

    /// Assume state is `(old_id << ID_SHIFT) | SOME`
    ///
    /// Update state to `(new_id << ID_SHIFT) | LOCK | NONE`
    #[inline]
    pub unsafe fn try_lock_some(&self, old_id: usize, new_id: usize) -> Result<usize, usize> {
        self.state.compare_exchange(
            (old_id << ID_SHIFT) | SOME,
            (new_id << ID_SHIFT) | LOCK | NONE,
            AcqRel,
            Acquire,
        )
    }

    /// Assume state is `(id << ID_SHIFT) | LOCK | NONE`
    ///
    /// Update state to `(id << ID_SHIFT) | NONE`
    #[inline]
    pub unsafe fn unchecked_get(&self, id: usize) -> T {
        let value = (*self.value.get()).assume_init_read();
        self.state.store((id << ID_SHIFT) | NONE, Release);
        value
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.state.load(Acquire) & STATE_MASK == NONE
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        self.state.load(Acquire) & STATE_MASK == SOME
    }
}

impl<T> Default for IdSlot<T> {
    #[inline]
    fn default() -> Self {
        Self::new(0)
    }
}

impl<T> Drop for IdSlot<T> {
    fn drop(&mut self) {
        if self.state.load(Acquire) & STATE_MASK == SOME {
            unsafe { self.value.get_mut().assume_init_drop() }
        }
    }
}

impl<T> fmt::Debug for IdSlot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.load(Acquire);
        f.debug_struct("Slab")
            .field(
                "state",
                &match state & STATE_MASK {
                    NONE => "NONE",
                    SOME => "SOME",
                    state if state == LOCK | NONE => "LOCK|NONE",
                    state if state == LOCK | SOME => "LOCK|SOME",
                    _ => unreachable!("invalid state"),
                },
            )
            .field("id", &(state >> ID_SHIFT))
            .field("value", &"..")
            .finish()
    }
}
