pub mod single;
pub use single::Single;
pub mod bounded;
pub use bounded::Bounded;

pub mod prelude {
    pub use super::bounded::Bounded;
    pub use super::single::Single;
    pub use super::Queue;
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
