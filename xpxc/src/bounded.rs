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
        let mut tail = self.tail.load(Acquire);
        let mut slot = &self.slab[tail % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_none(tail) } {
                Ok(_) => {
                    self.tail.store((tail + 1) & ID_MOD, Release);
                    unsafe { slot.unchecked_set(value, tail) };
                    return Ok(());
                }
                Err(state) => {
                    let id = state >> ID_SHIFT;
                    let state = state & STATE_MASK;

                    if id != tail {
                        return Err(value);
                    }

                    match state {
                        SOME => {
                            let new_tail = self.tail.load(Acquire);
                            if tail == new_tail {
                                return Err(value);
                            }
                            tail = new_tail;
                            slot = &self.slab[tail % self.slab.len()];
                        }
                        LOCKNONE => {}
                        LOCKSOME => {}
                        _ => {
                            unreachable!();
                        }
                    }
                }
            }
        }
    }

    fn pop(&self) -> Result<T, ()> {
        let mut head = self.head.load(Acquire);
        let mut slot = &self.slab[head % self.slab.len()];

        loop {
            match unsafe { slot.try_lock_some(head) } {
                Ok(_) => {
                    self.head.store((head + 1) & ID_MOD, Release);
                    return Ok(unsafe { slot.unchecked_get(head + self.slab.len()) });
                }
                Err(state)=>{
                    let id = state >> ID_SHIFT;
                    let state = state & STATE_MASK;

                    if id != head {
                        return Err(());
                    }

                    match state {
                        NONE => {
                            let new_head = self.head.load(Acquire);
                            if head == new_head {
                                return Err(());
                            }
                            head = new_head;
                            slot = &self.slab[head % self.slab.len()];
                        }
                        LOCKSOME => {}
                        LOCKNONE => {}
                        _ => {
                            unreachable!();
                        }
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
