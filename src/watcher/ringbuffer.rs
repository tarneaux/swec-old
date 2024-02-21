use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A fixed length ring buffer that overwrites the oldest element when full.
#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    // The inner buffer. Using its capacity should be avoided, because it may
    // be more than our capacity.
    inner: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer of the given capacity.
    /// # Example
    /// ```
    /// use swec::watcher::RingBuffer;
    /// let rb = RingBuffer::<i32>::new(5);
    /// assert_eq!(rb.capacity(), 5);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add an element to the ring buffer, overwriting the oldest element if full.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(1);
    /// rb.push(1);
    /// rb.push(2);
    /// assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![2]);
    /// ```
    pub fn push(&mut self, elem: T) {
        if self.inner.len() == self.capacity {
            self.inner.pop_front();
        }
        self.inner.push_back(elem);
    }

    /// Add multiple elements to the ring buffer, overwriting the oldest elements if full.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(3);
    /// rb.push_multiple(1..=10);
    /// assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![8, 9, 10]);
    /// ```
    pub fn push_multiple<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for elem in iter {
            self.push(elem);
        }
    }

    /// Get an iterator over the elements in the ring buffer.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(3);
    /// rb.push_multiple(1..=10);
    /// let iter = rb.iter();
    /// assert_eq!(iter.copied().collect::<Vec<_>>(), vec![8, 9, 10]);
    /// ```
    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.inner.iter()
    }

    /// Get the capacity of the ring buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the length of the ring buffer.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Dynamically increase the capacity of the ring buffer.
    /// The new capacity must be greater than or equal to the current length of the buffer.
    /// Also grows the buffer to the new capacity if necessary.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(3);
    /// rb.resize(5).unwrap();
    /// rb.resize(4).unwrap_err(); // Does nothing
    /// rb.push_multiple(1..=10);
    /// assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![6, 7, 8, 9, 10]);
    /// ```
    pub fn resize(&mut self, capacity: usize) -> Result<()> {
        if capacity < self.capacity {
            Err(eyre!(
                "New capacity is less than the current length of the buffer."
            ))
        } else {
            self.inner.reserve(capacity - self.capacity);
            self.capacity = capacity;
            Ok(())
        }
    }

    /// Dynamically change the capacity of the ring buffer, deleting oldest elements as necessary.
    /// Also shrinks or grows the buffer to the new capacity if needed.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(3);
    /// rb.truncate_fifo(5);
    /// rb.push_multiple(1..=10);
    /// assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![6, 7, 8, 9, 10]);
    /// rb.truncate_fifo(3);
    /// assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![8, 9, 10]);
    /// ```
    pub fn truncate_fifo(&mut self, capacity: usize) {
        if capacity < self.capacity {
            while self.inner.len() > capacity {
                self.inner.pop_front();
            }
            self.inner.shrink_to_fit();
        } else if capacity > self.capacity {
            self.inner.reserve(capacity - self.capacity);
        }
        self.capacity = capacity;
    }
}

impl<T> Serialize for RingBuffer<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for RingBuffer<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        VecDeque::deserialize(deserializer).map(RingBuffer::from)
    }
}

impl<T> From<VecDeque<T>> for RingBuffer<T> {
    fn from(inner: VecDeque<T>) -> Self {
        let capacity = inner.len(); // Not capacity: it may be more than length
        Self { inner, capacity }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let rb = RingBuffer::<i32>::new(5);
        assert_eq!(rb.capacity(), 5);
    }

    #[test]
    fn test_push() {
        let mut rb = RingBuffer::<i32>::new(5);
        rb.push(1);
        rb.push(2);
        rb.push(3);
        rb.push(4);
        rb.push(5);
        rb.push(6);
        rb.push(7);
        rb.push(8);
        rb.push(9);
        rb.push(10);
        assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_push_multiple() {
        let mut rb = RingBuffer::<i32>::new(5);
        rb.push_multiple(1..=10);
        assert_eq!(rb.iter().copied().collect::<Vec<_>>(), vec![6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_serialize() {
        let rb = RingBuffer::<i32>::new(5);
        let serialized = serde_json::to_string(&rb).unwrap();
        assert_eq!(serialized, "[]");
    }

    #[test]
    fn test_deserialize() {
        let rb: RingBuffer<i32> = serde_json::from_str("[1,2,3,4,5,6,7]").unwrap();
        assert_eq!(
            rb.iter().copied().collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7]
        );
        assert_eq!(rb.capacity(), 7);
    }
}
