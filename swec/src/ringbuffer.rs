use core::fmt::{self, Debug, Formatter};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::VecDeque};
use swec_core::{Status, StatusBuffer};

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
    #[must_use]
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
    /// The first element is the oldest, and the last element is the newest.
    /// # Example
    /// ```
    /// # use swec::watcher::RingBuffer;
    /// let mut rb = RingBuffer::<i32>::new(3);
    /// rb.push_multiple(1..=10);
    /// let iter = rb.iter();
    /// assert_eq!(iter.copied().collect::<Vec<_>>(), vec![8, 9, 10]);
    /// ```
    #[must_use]
    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.inner.iter()
    }

    /// Get the capacity of the ring buffer.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the length of the ring buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the ring buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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
    /// # Errors
    /// Returns a `ResizeError` if the new capacity is less than the current length of the buffer.
    pub fn resize(&mut self, capacity: usize) -> Result<(), ResizeError> {
        if capacity < self.capacity {
            Err(ResizeError {
                new_capacity: capacity,
                length: self.inner.len(),
            })
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
        match capacity.cmp(&self.capacity) {
            Ordering::Less => {
                while self.inner.len() > capacity {
                    self.inner.pop_front();
                }
                self.inner.shrink_to_fit();
            }
            Ordering::Greater => {
                self.inner.reserve(capacity - self.capacity);
            }
            Ordering::Equal => {}
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
        VecDeque::deserialize(deserializer).map(Self::from)
    }
}

impl<T> From<VecDeque<T>> for RingBuffer<T> {
    fn from(inner: VecDeque<T>) -> Self {
        let capacity = inner.len(); // Not capacity: it may be more than length
        Self { inner, capacity }
    }
}

impl<T> Iterator for RingBuffer<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.pop_front()
    }
}

impl<T> DoubleEndedIterator for RingBuffer<T>
where
    T: Clone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.pop_back()
    }
}

impl<'a, T> IntoIterator for &'a RingBuffer<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

pub struct ResizeError {
    new_capacity: usize,
    length: usize,
}

impl Debug for ResizeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "New capacity ({}) is less than the current length of the buffer ({}).",
            self.new_capacity, self.length
        )
    }
}

pub type StatusRingBuffer = RingBuffer<(chrono::DateTime<chrono::Local>, Status)>;

impl StatusBuffer for StatusRingBuffer {
    fn push(&mut self, status: (chrono::DateTime<chrono::Local>, Status)) {
        self.push(status);
    }

    fn get(&self, index: usize) -> Option<(chrono::DateTime<chrono::Local>, Status)> {
        // Using the inner buffer directly avoids recursion.
        self.inner.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let rb = RingBuffer::<i32>::new(5);
        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 0);
        assert!(rb.is_empty());
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
        assert_eq!(
            (&rb).into_iter().collect::<Vec<_>>(),
            vec![&6, &7, &8, &9, &10]
        );
        assert_eq!(rb.capacity(), 5);
        assert_eq!(rb.len(), 5);
        assert!(!rb.is_empty());
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

    #[test]
    fn test_rev_iter() {
        let mut rb = RingBuffer::<i32>::new(5);
        rb.push_multiple(1..=10);
        let iter = rb.iter().rev();
        assert_eq!(iter.copied().collect::<Vec<_>>(), vec![10, 9, 8, 7, 6]);
        let iter = rb.rev();
        assert_eq!(iter.collect::<Vec<_>>(), vec![10, 9, 8, 7, 6]);
    }
}
