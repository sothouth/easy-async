pub trait Queue<T> {
    /// Reture `Some(value)` if the queue is full.
    fn push(&self, value: T) -> Option<T>;
    fn pop(&self) -> Option<T>;
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
