use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::fmt;
use std::task::Waker;

const NOOP: &'static Waker = Waker::noop();

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
    pub fn take(&self) -> Waker {
        self.replace(NOOP.clone())
    }

    #[inline]
    pub fn register(&self, waker: &Waker) {
        unsafe { (*self.0.get()).clone_from(waker) }
    }

    #[inline]
    pub fn will_wake<T: Borrow<Waker>>(&self, other: &T) -> bool {
        unsafe { (*self.0.get()).will_wake(other.borrow()) }
    }

    // not right.
    // the Waker::noop()'s vtable have a pointer,
    // and the any clone of Waker::noop()'s vtable have another same pointer,
    // don't know why
    // self.will_wake(NOOP.clone()) will return true result
    // #[inline]
    // pub fn is_noop(&self) -> bool {
    //     // self.will_wake(NOOP)
    //     unsafe { std::ptr::eq((*self.0.get()).as_raw().vtable(), NOOP.as_raw().vtable()) }
    // }

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
