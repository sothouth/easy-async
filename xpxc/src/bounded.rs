use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

use super::Queue;
use super::Slot;

use super::LOCK;
use super::NONE;
use super::SOME;

const LOCKNONE: usize = LOCK | NONE;
const LOCKSOME: usize = LOCK | SOME;

pub struct Bounded<T> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slab: Box<[Slot<T>]>,
}

unsafe impl<T: Send> Send for Bounded<T> {}
unsafe impl<T: Send> Sync for Bounded<T> {}

impl<T> Bounded<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be positive");

        let mut slab = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            slab.push(Slot::new());
        }

        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            slab: slab.into_boxed_slice(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(capacity)
    }
}

impl<T> Queue<T> for Bounded<T> {
    fn push(&self, value: T) -> Result<(), T> {
        let tail = self.tail.fetch_add(1, AcqRel);
        let slot = &self.slab[tail % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_none() } {
                Ok(_) => {
                    unsafe { slot.unchecked_set(value) };
                    return Ok(());
                }
                Err(LOCKNONE) => {}
                // SOME or LOCK|SOME
                Err(_) => {
                    if tail.wrapping_sub(self.head.load(Acquire)) >= self.slab.len() {
                        self.tail.fetch_sub(1, AcqRel);
                        return Err(value);
                    }
                }
            }
        }
    }

    fn pop(&self) -> Result<T, ()> {
        let head = self.head.fetch_add(1, AcqRel);
        let slot = &self.slab[head % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_some() } {
                Ok(_) => {
                    return Ok(unsafe { slot.unchecked_get() });
                }
                Err(LOCKSOME) => {}
                // NONE or LOCK|NONE
                Err(_) => {
                    // if head >= self.tail.load(Acquire) {
                        if self.tail.load(Acquire).wrapping_sub(head+1)>=self.slab.len() {
                        self.head.fetch_sub(1, AcqRel);
                        return Err(());
                    }
                }
            }
        }
    }

    fn len(&self) -> usize {
        let head = self.head.load(Acquire);
        let tail = self.tail.load(Acquire);

        let len = tail.wrapping_sub(head);

        if len < self.slab.len() {
            len
        } else {
            self.slab.len()
        }
    }

    fn capacity(&self) -> usize {
        self.slab.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    fn slack(&self) -> usize {
        self.capacity() - self.len()
    }
}
