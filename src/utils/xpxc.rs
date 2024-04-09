pub mod single;
pub use single::Single;

pub fn bounded<T>(capacity: usize) -> impl Queue<T> {
    if capacity == 1 {
        Single::new()
    } else {
        todo!()
    }
}

pub fn unbounded<T>() -> impl Queue<T> {
    todo!();
    Single::new()
}

pub trait Queue<T> {
    /// Reture `Some(value)` if the queue is full.
    fn push(&self, value: T) -> Result<(), T>;
    fn pop(&self) -> Result<T, ()>;
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }
    /// The remaining capacity.
    fn slack(&self) -> usize {
        self.capacity() - self.len()
    }
}
