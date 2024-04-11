use super::Queue;
use super::Slot;

#[repr(transparent)]
pub struct Single<T> {
    slot: Slot<T>,
}

unsafe impl<T: Send> Send for Single<T> {}
unsafe impl<T: Send> Sync for Single<T> {}

impl<T> Single<T> {
    pub fn new() -> Self {
        Self { slot: Slot::new() }
    }
}

impl<T> Queue<T> for Single<T> {
    fn push(&self, value: T) -> Result<(), T> {
        self.slot.set(value)
    }

    fn pop(&self) -> Result<T, ()> {
        self.slot.get()
    }

    #[inline]
    fn len(&self) -> usize {
        self.slot.is_some() as usize
    }

    #[inline]
    fn capacity(&self) -> usize {
        1
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.slot.is_none()
    }

    #[inline]
    fn is_full(&self) -> bool {
        self.slot.is_some()
    }

    #[inline]
    fn slack(&self) -> usize {
        self.slot.is_none() as usize
    }
}