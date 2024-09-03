use std::fmt;
use std::ops::Rem;

/// A ring buffer with a fixed size `N`. When the buffer is full, new values
/// will overwrite the oldest values, starting from the beginning of the buffer.
pub struct RingBuffer<T, const N: usize> {
    inner: [T; N],
    idx: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    /// Creates a new `RingBuffer` with the given buffer.
    pub const fn new(buff: [T; N]) -> Self {
        Self {
            inner: buff,
            idx: 0,
        }
    }

    /// Pushes a value into the ring buffer. If the buffer is full,
    /// this will overwrite the oldest value.
    pub fn push(&mut self, value: T) {
        self.inner[self.idx] = value;
        self.idx = (self.idx + 1).rem(N);
    }

    /// Returns an iterator over the elements of the buffer.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// Returns a reference to the element at the given index, or
    /// `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    /// Returns a mutable reference to the element at the given index, or
    /// `None` if the index is out of bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }

    /// Returns a reference to the last element that was added to the buffer,
    /// or `None` if the buffer is empty.
    pub fn last(&self) -> Option<&T> {
        if self.idx == 0 {
            self.inner.get(N - 1)
        } else {
            self.inner.get(self.idx - 1)
        }
    }

    /// Returns the length of the ring buffer, which is fixed at `N`.
    pub fn len(&self) -> usize {
        N
    }

    /// Returns if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T, const N: usize> Default for RingBuffer<T, N>
where
    T: Default + Copy,
{
    fn default() -> Self {
        Self::new([Default::default(); N])
    }
}

impl<T: Copy, const N: usize> Copy for RingBuffer<T, N> {}

impl<T: Clone, const N: usize> Clone for RingBuffer<T, N> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            idx: self.idx,
        }
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for RingBuffer<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RingBuffer")
            .field("inner", &self.inner)
            .field("idx", &self.idx)
            .finish()
    }
}

impl<T, const N: usize> IntoIterator for RingBuffer<T, N> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::RingBuffer;

    #[test]
    fn push_and_get() {
        let mut buffer = RingBuffer::new([0; 5]);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.get(0), Some(&1));
        assert_eq!(buffer.get(1), Some(&2));
        assert_eq!(buffer.get(2), Some(&3));
        assert_eq!(buffer.get(3), Some(&0)); // Unchanged
    }

    #[test]
    fn overwrite() {
        let mut buffer = RingBuffer::new([0; 3]);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4); // Overwrites the first element
        assert_eq!(buffer.get(0), Some(&4));
        assert_eq!(buffer.get(1), Some(&2));
        assert_eq!(buffer.get(2), Some(&3));
    }

    #[test]
    fn last() {
        let mut buffer = RingBuffer::new([0; 3]);
        buffer.push(1);
        buffer.push(2);
        assert_eq!(buffer.last(), Some(&2));
        buffer.push(3);
        assert_eq!(buffer.last(), Some(&3));
        buffer.push(4); // Overwrites the first element
        assert_eq!(buffer.last(), Some(&4));
    }

    #[test]
    fn iter() {
        let buffer = RingBuffer::new([1, 2, 3, 4, 5]);
        let collected: Vec<_> = buffer.iter().collect();
        assert_eq!(collected, vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn default() {
        let buffer: RingBuffer<i32, 3> = Default::default();
        assert_eq!(buffer.get(0), Some(&0));
        assert_eq!(buffer.get(1), Some(&0));
        assert_eq!(buffer.get(2), Some(&0));
    }

    #[test]
    fn copy() {
        let buffer = RingBuffer::new([1, 2, 3]);
        let _copy = buffer; // Copy the buffer
        assert_eq!(buffer.get(0), Some(&1)); // Original buffer is still valid
        assert_eq!(buffer.get(1), Some(&2));
        assert_eq!(buffer.get(2), Some(&3));
    }

    #[test]
    fn clone() {
        let buffer = RingBuffer::new([1, 2, 3]);
        let clone = buffer; // Clone the buffer
        assert_eq!(clone.get(0), Some(&1)); // Clone has the same contents
        assert_eq!(clone.get(1), Some(&2));
        assert_eq!(clone.get(2), Some(&3));
        assert_eq!(buffer.last(), Some(&3)); // Original buffer is unchanged
    }
}
