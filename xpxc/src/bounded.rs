use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

use super::Queue;
use super::Slot;

use super::LOCK;
use super::NONE;
use super::SOME;

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
        let mut tail = self.tail.load(Acquire);

        loop {
            let slot = &self.slab[tail];

            match unsafe { slot.try_lock_none() } {
                Ok(_) => {
                    self.tail.store((tail + 1) % self.slab.len(), Release);
                    unsafe { slot.unchecked_set(value) };
                    return Ok(());
                }
                // tail has been updated
                Err(SOME) => {
                    let new_tail = self.tail.load(Acquire);
                    if tail == new_tail {
                        return Err(value);
                    }
                    tail = new_tail;
                }
                Err(_) => {
                    if tail == self.head.load(Acquire) {
                        return Err(value);
                    }
                    tail = self.tail.load(Acquire);
                }
            }
        }
    }

    fn pop(&self) -> Result<T, ()> {
        let mut head = self.head.load(Acquire);

        loop {
            let slot = &self.slab[head];

            match unsafe { slot.try_lock_some() } {
                Ok(_) => {
                    self.head.store((head + 1) % self.slab.len(), Release);
                    return Ok(unsafe { slot.unchecked_get() });
                }
                // head has been updated
                Err(NONE) => {
                    let new_head = self.head.load(Acquire);
                    if head == new_head {
                        return Err(());
                    }
                    head = new_head;
                }
                Err(_) => {
                    if head == self.tail.load(Acquire) {
                        return Err(());
                    }
                    head = self.head.load(Acquire);
                }
            }
        }
    }

    fn len(&self) -> usize {
        let head = self.head.load(Acquire);
        let tail = self.tail.load(Acquire);

        let len = tail.wrapping_sub(head).wrapping_add(self.slab.len()) % self.slab.len();

        if len == 0 && self.slab[tail].is_some() {
            self.slab.len()
        } else {
            len
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
