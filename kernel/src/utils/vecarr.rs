pub struct VecArr<T, const LEN: usize> {
    array: [T; LEN],
    length: usize,
}

impl<T, const LEN: usize> VecArr<T, LEN> {
    pub fn new(init: impl FnMut(usize) -> T) -> Self {
        Self {
            array: core::array::from_fn(init),
            length: 0,
        }
    }
    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.length < LEN {
            self.array[self.length] = value;
            self.length += 1;
            Ok(())
        } else {
            Err(value)
        }
    }
    pub fn as_slice(&self) -> &[T] {
        &self.array[..self.length]
    }
}
