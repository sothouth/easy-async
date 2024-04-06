use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::task::{RawWakerVTable, Waker};
use std::{fmt, ptr};

const NOOP: &'static Waker = Waker::noop();

/// Another version of `UnsafeCell<Option<Waker>>`, maybe faster.
#[repr(transparent)]
pub struct OptionWaker(UnsafeCell<Waker>);

impl OptionWaker {
    #[inline]
    pub fn new() -> Self {
        Self(UnsafeCell::new(NOOP.clone()))
    }

    #[inline]
    pub fn wake(&self) {
        self.take().wake();
    }

    #[inline]
    pub fn wake_by_ref(&self) {
        unsafe { (*self.0.get()).wake_by_ref() };
    }

    /// Take the old waker and leave a NOOP waker.
    #[inline]
    pub fn take(&self) -> Waker {
        self.replace(NOOP.clone())
    }

    /// The new waker will replace the old waker.
    #[inline]
    pub fn register(&self, waker: &Waker) {
        unsafe { (*self.0.get()).clone_from(waker) }
    }

    /// Like `Waker::will_wake`.
    #[inline]
    pub fn will_wake<T: Borrow<Waker>>(&self, other: &T) -> bool {
        unsafe { (*self.0.get()).will_wake(other.borrow()) }
    }

    /// Check if the waker is a NOOP waker, slightly slow.
    #[inline]
    pub fn is_noop(&self) -> bool {
        // NOOP.as_raw().vtable() is not right.
        // const RawWaker::NOOP have two copy
        // the Waker::noop()'s vtable have the ptr inside Waker::noop()
        // any clone of Waker::noop()'s vtable ptr equal to RawWaker::NOOP.vtable
        let ptr: &'static RawWakerVTable = NOOP.clone().as_raw().vtable();
        unsafe { ptr::eq((*self.0.get()).as_raw().vtable(), ptr) }
    }

    #[inline]
    fn replace(&self, waker: Waker) -> Waker {
        unsafe { self.0.get().replace(waker) }
    }
}

impl Default for OptionWaker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for OptionWaker {
    fn clone(&self) -> Self {
        Self(UnsafeCell::new(unsafe { (*self.0.get()).clone() }))
    }
}

impl From<Waker> for OptionWaker {
    fn from(waker: Waker) -> Self {
        Self(UnsafeCell::new(waker))
    }
}

impl fmt::Debug for OptionWaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Option")?;
        unsafe { (*self.0.get()).fmt(f) }
    }
}

impl Borrow<Waker> for OptionWaker {
    fn borrow(&self) -> &Waker {
        unsafe { &*self.0.get() }
    }
}
