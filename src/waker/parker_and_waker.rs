//! A poor imitation of Parking.
//! use parking as _;

use std::sync::{
    atomic::{AtomicUsize, Ordering::*},
    Arc, Condvar, Mutex,
};

use std::cell::Cell;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::task::{Wake, Waker};
use std::time::{Duration, Instant};

#[inline]
pub fn parker_and_waker() -> (Parker, Waker) {
    let (parker, unparker) = pair();
    (parker, Waker::from(unparker))
}

#[inline]
pub fn pair() -> (Parker, Unparker) {
    let p = Parker::new();
    let u = p.unparker();
    (p, u)
}

pub struct Parker {
    unparker: Unparker,
    _marker: PhantomData<Cell<()>>,
}

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

    pub fn park(&self) {
        self.unparker.inner.park();
    }

    pub fn park_timeout(&self, timeout: Duration) -> bool {
        self.unparker.inner.park_timeout(timeout)
    }

    pub fn park_deadline(&self, instant: Instant) -> bool {
        self.unparker
            .inner
            .park_timeout(instant.saturating_duration_since(Instant::now()))
    }

    pub fn unpark(&self) -> bool {
        self.unparker.inner.unpark()
    }

    pub fn unparker(&self) -> Unparker {
        self.unparker.clone()
    }
}

impl Default for Parker {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Parker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad("Parker { .. }")
    }
}

impl Unparker {
    pub fn unpark(&self) -> bool {
        self.inner.unpark()
    }

    pub fn same_parker(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

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

impl Debug for Unparker {
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
