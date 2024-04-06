


pub struct CallOnDrop<F: FnMut()>(pub F);

impl<F: FnMut()> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        (self.0)()
    }
}
