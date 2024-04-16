use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

use super::IdSlot;
use super::Queue;

use super::LOCK;
use super::NONE;
use super::SOME;

use super::ID_MOD;
use super::ID_SHIFT;
use super::STATE_MASK;

const LOCKNONE: usize = LOCK | NONE;
const LOCKSOME: usize = LOCK | SOME;

pub struct Bounded<T> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slab: Box<[IdSlot<T>]>,
}

unsafe impl<T: Send> Send for Bounded<T> {}
unsafe impl<T: Send> Sync for Bounded<T> {}

impl<T> Bounded<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be positive");

        let mut slab = Vec::with_capacity(capacity);
        for ith in 0..capacity {
            slab.push(IdSlot::new(ith));
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
        let mut tail = self.tail.fetch_add(1, AcqRel) & ID_MOD;
        let mut slot = &self.slab[tail % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_none(tail) } {
                Ok(_) => {
                    unsafe { slot.unchecked_set(value, tail) };
                    return Ok(());
                }
                Err(state) => {
                    let id = state >> ID_SHIFT;
                    let state = state & STATE_MASK;

                    if id == tail {
                        continue;
                    } else if tail.wrapping_sub(id) % self.slab.len() == 0 {
                        self.tail.fetch_sub(1, AcqRel);
                        return Err(value);
                    } else {
                        unreachable!("{} {} {}", id, tail, self.slab.len());
                        // self.tail.fetch_sub(1, AcqRel);
                        // return Err(value);
                    }
                }
            }
        }
    }

    fn pop(&self) -> Result<T, ()> {
        let mut head = self.head.fetch_add(1, AcqRel) & ID_MOD;
        let mut slot = &self.slab[head % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_some(head, head + self.slab.len()) } {
                Ok(_) => {
                    return Ok(unsafe { slot.unchecked_get(head + self.slab.len()) });
                }
                Err(state) => {
                    let id = state >> ID_SHIFT;
                    let state = state & STATE_MASK;

                    if id == head {
                        match state {
                            NONE | LOCKNONE => {
                                self.head.fetch_sub(1, AcqRel);
                                return Err(());
                            }
                            LOCKSOME => {}
                            _ => {
                                unreachable!();
                            }
                        }
                    } else {
                        self.head.fetch_sub(1, AcqRel);
                        return Err(());
                        // match state{
                        //     NONE|LOCKNONE=>{}

                        // }
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
