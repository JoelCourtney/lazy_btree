pub struct VecSlicer<'a, T> {
    slice_start: usize,
    current_index: usize,
    vec: &'a mut Vec<T>,
}

impl<'a, T> VecSlicer<'a, T> {
    pub fn new(vec: &'a mut Vec<T>) -> Self {
        Self {
            slice_start: 0,
            current_index: 0,
            vec,
        }
    }

    pub fn advance(&mut self, count: usize) {
        self.current_index += count;
    }

    pub fn slice(&mut self) -> SliceThief<T> {
        let thief = SliceThief {
            start: &self.vec[self.slice_start],
            current: 0,
            len: self.current_index - self.slice_start,
        };
        self.slice_start = self.current_index;
        thief
    }

    pub fn take(&mut self) -> T {
        debug_assert_eq!(self.slice_start, self.current_index);
        let result = unsafe { (&self.vec[self.current_index] as *const T).read() };
        self.slice_start += 1;
        self.current_index = self.slice_start;
        result
    }

    pub fn remaining(&self) -> usize {
        self.vec.len() - self.current_index
    }

    pub fn current(&self) -> &T {
        &self.vec[self.current_index]
    }

    pub fn slice_to_end(&mut self) -> SliceThief<T> {
        self.current_index = self.vec.len();
        self.slice()
    }
}

impl<T> Drop for VecSlicer<'_, T> {
    fn drop(&mut self) {
        debug_assert_eq!(self.current_index, self.vec.len());
        debug_assert_eq!(self.slice_start, self.vec.len());
        unsafe {
            self.vec.set_len(0);
        }
    }
}

pub struct SliceThief<T> {
    start: *const T,
    current: usize,
    len: usize,
}

impl<T> SliceThief<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn peek_first(&self) -> &T {
        debug_assert_ne!(self.len, 0, "Cannot peek first element of empty slice");
        debug_assert_eq!(
            self.current, 0,
            "Cannot peek first element when it has already been consumed."
        );
        unsafe { &*self.start }
    }

    pub fn peek_last(&self) -> &T {
        debug_assert_ne!(self.len, 0, "Cannot peek last element of empty slice");
        debug_assert_ne!(
            self.current, self.len,
            "Cannot peek last element when it has already been consumed."
        );
        unsafe { &*self.start.add(self.len - 1) }
    }
}

impl<T> Drop for SliceThief<T> {
    fn drop(&mut self) {
        debug_assert_eq!(self.current, self.len);
    }
}

impl<T> Iterator for SliceThief<T> {
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            let result = unsafe { Some(self.start.add(self.current).read()) };
            self.current += 1;
            result
        } else {
            None
        }
    }
}

impl<T> ExactSizeIterator for SliceThief<T> {}

impl<T> DoubleEndedIterator for SliceThief<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current < self.len {
            self.len -= 1;
            unsafe { Some(self.start.add(self.len).read()) }
        } else {
            None
        }
    }
}
