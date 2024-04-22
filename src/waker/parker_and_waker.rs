//! Imitation of `Parking`.

use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Wake, Waker};
use std::time::{Duration, Instant};

/// Creates a parker and an associated waker.
///
/// # Examples
///
/// ```ignore
/// let (p, w) = easy_async::waker::parker_and_waker();
/// ```
#[inline]
pub fn parker_and_waker() -> (Parker, Waker) {
    let (parker, unparker) = pair();
    (parker, Waker::from(unparker))
}

/// Creates a parker and an associated unparker.
///
/// # Examples
///
/// ```ignore
/// let (p, u) = easy_async::waker::pair();
/// ```.
#[inline]
pub fn pair() -> (Parker, Unparker) {
    let p = Parker::new();
    let u = p.unparker();
    (p, u)
}

/// Waits for a notification.
pub struct Parker {
    unparker: Unparker,
    _marker: PhantomData<Cell<()>>,
}

/// Notifies a parker.
pub struct Unparker {
    inner: Arc<Inner>,
}

struct Inner {
    state: AtomicUsize,
    lock: Mutex<()>,
    cvar: Condvar,
}

const EMPTY: usize = 0;
const PARKED: usize = 1;
const NOTIFIED: usize = 2;

impl Parker {
    /// Creates a new parker.
    ///
    /// # Examples
    ///
    /// ```
    /// use easy_async::waker::Parker;
    ///
    /// let p = Parker::new();
    /// ```
    ///
    pub fn new() -> Self {
        Self {
            unparker: Unparker {
                inner: Arc::new(Inner {
                    state: AtomicUsize::new(EMPTY),
                    lock: Mutex::new(()),
                    cvar: Condvar::new(),
                }),
            },
            _marker: PhantomData,
        }
    }

    /// Blocks until notified and then goes back into unnotified state.
    pub fn park(&self) {
        self.unparker.inner.park();
    }

    /// Blocks until notified and then goes back into unnotified state,
    /// or times out after `duration`.
    pub fn park_timeout(&self, timeout: Duration) -> bool {
        self.unparker.inner.park_timeout(timeout)
    }

    /// Blocks until notified and then goes back into unnotified state,
    /// or times out at `instant`.
    pub fn park_deadline(&self, instant: Instant) -> bool {
        self.unparker
            .inner
            .park_timeout(instant.saturating_duration_since(Instant::now()))
    }

    /// Notifies the parker.
    ///
    /// Returns `true` if this call is the first to notify the parker,
    /// or `false` if the parker was already notified.
    pub fn unpark(&self) -> bool {
        self.unparker.inner.unpark()
    }

    /// Returns a handle for unparking.
    ///
    /// The returned [`Unparker`] can be cloned and shared among threads.
    pub fn unparker(&self) -> Unparker {
        self.unparker.clone()
    }
}

impl Default for Parker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Parker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Parker { .. }")
    }
}

impl Unparker {
    /// Notifies the associated parker.
    ///
    /// Returns `true` if this call is the first to notify the parker,
    /// or `false` if the parker was already notified.
    pub fn unpark(&self) -> bool {
        self.inner.unpark()
    }

    /// Indicates whether two unparkers will unpark the same parker.
    pub fn same_parker(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Indicates whether this unparker will unpark the associated parker.
    pub fn will_unpark(&self, other: &Parker) -> bool {
        Arc::ptr_eq(&self.inner, &other.unparker.inner)
    }
}

impl Clone for Unparker {
    fn clone(&self) -> Self {
        Unparker {
            inner: self.inner.clone(),
        }
    }
}

impl From<Unparker> for Waker {
    fn from(unparker: Unparker) -> Self {
        Waker::from(unparker.inner)
    }
}

impl fmt::Debug for Unparker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Unparker { .. }")
    }
}

impl Inner {
    fn park(&self) {
        if self
            .state
            .compare_exchange(NOTIFIED, EMPTY, AcqRel, Acquire)
            .is_ok()
        {
            return;
        }

        let mut mutex = self.lock.lock().unwrap();

        match self.state.compare_exchange(EMPTY, PARKED, AcqRel, Acquire) {
            Ok(_) => (),
            Err(NOTIFIED) => {
                self.state.swap(EMPTY, AcqRel);
                return;
            }
            Err(_) => panic!("inconsistent state in park"),
        }

        loop {
            mutex = self.cvar.wait(mutex).unwrap();

            if self
                .state
                .compare_exchange(NOTIFIED, EMPTY, AcqRel, Acquire)
                .is_ok()
            {
                return;
            }
        }
    }

    fn park_timeout(&self, timeout: Duration) -> bool {
        if self
            .state
            .compare_exchange(NOTIFIED, EMPTY, AcqRel, Acquire)
            .is_ok()
        {
            return true;
        }

        if timeout.is_zero() {
            return false;
        }

        let mutex = self.lock.lock().unwrap();

        match self.state.compare_exchange(EMPTY, PARKED, AcqRel, Acquire) {
            Ok(_) => (),
            Err(NOTIFIED) => {
                self.state.store(EMPTY, Release);
                return true;
            }
            Err(_) => panic!("inconsistent state in park"),
        }

        _ = self.cvar.wait_timeout(mutex, timeout).unwrap();
        match self.state.swap(EMPTY, AcqRel) {
            NOTIFIED => true,
            PARKED => false,
            _ => panic!("inconsistent state in park_timeout"),
        }
    }

    fn unpark(&self) -> bool {
        match self.state.swap(NOTIFIED, AcqRel) {
            EMPTY => return true,
            NOTIFIED => return false,
            PARKED => (),
            _ => panic!("inconsistent state in unpark"),
        }

        drop(self.lock.lock().unwrap());
        self.cvar.notify_one();
        true
    }
}

impl Wake for Inner {
    #[inline]
    fn wake(self: Arc<Self>) {
        self.unpark();
    }

    #[inline]
    fn wake_by_ref(self: &Arc<Self>) {
        self.unpark();
    }
}
