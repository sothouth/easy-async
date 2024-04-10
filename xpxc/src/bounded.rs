use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;

use super::Queue;

struct Slab<T> {
    state: AtomicUsize,
    value: UnsafeCell<MaybeUninit<T>>,
}

pub struct Bounded<T> {
    head: AtomicUsize,
    tail: AtomicUsize,
    buffer: Box<[Slab<T>]>,
}

unsafe impl<T: Send> Send for Bounded<T> {}
unsafe impl<T: Send> Sync for Bounded<T> {}

impl<T> Bounded<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must > 0");
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            buffer: Vec::with_capacity(capacity).into_boxed_slice(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(capacity)
    }
}

impl<T> Queue<T> for Bounded<T> {
    fn push(&self, value: T) -> Result<(), T> {
        todo!()
    }

    fn pop(&self) -> Result<T, ()> {
        todo!()
    }

    fn len(&self) -> usize {
        todo!()
    }

    fn capacity(&self) -> usize {
        todo!()
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

impl<T> Drop for Bounded<T> {
    fn drop(&mut self) {}
}
