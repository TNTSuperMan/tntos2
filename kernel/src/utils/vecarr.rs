pub struct VecArr<T: Default, const LEN: usize> {
    array: [T; LEN],
    length: usize,
}

impl<T: Default, const LEN: usize> VecArr<T, LEN> {
    pub fn new() -> Self {
        Self {
            array: core::array::from_fn(|_| T::default()),
            length: 0,
        }
    }
    pub fn clear(&mut self) {
        self.length = 0;
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
    pub fn get(&self, index: usize) -> Option<&T> {
        if self.length > index {
            Some(&self.array[index])
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.length > index {
            Some(&mut self.array[index])
        } else {
            None
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.array.iter().take(self.length)
    }
}
