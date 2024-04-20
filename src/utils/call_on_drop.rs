/// Call a function when it goes out of scope.
pub struct CallOnDrop<F: FnMut()>(pub F);

impl<F: FnMut()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}
