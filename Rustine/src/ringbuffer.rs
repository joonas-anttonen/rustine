use std::ops::{Add, Div};

/// A simple ring buffer implementation that stores elements in a circular manner.
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    head: u32,
    count: u32,
}

impl<T: Clone + Default> RingBuffer<T> {
    /// Creates a new ring buffer with the specified capacity.
    ///
    /// # Panics
    /// Panics if capacity is 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "capacity must be at least 1");

        Self {
            buffer: vec![T::default(); capacity],
            head: 0,
            count: 0,
        }
    }

    /// Returns the maximum number of elements the ring buffer can hold.
    pub fn capacity(&self) -> u32 {
        self.buffer.len() as u32
    }

    /// Returns the current number of elements in the ring buffer.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Returns true if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clears all elements from the ring buffer.
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
        self.buffer.fill(T::default());
    }

    /// Pushes an item onto the ring buffer, overwriting the oldest element if full.
    pub fn push(&mut self, item: T) {
        self.buffer[self.head as usize] = item;
        self.head = (self.head + 1) % self.buffer.len() as u32;
        self.count = (self.count + 1).min(self.buffer.len() as u32);
    }

    pub fn min_max_mean(&self) -> Option<(T, T, T)>
    where
        T: PartialOrd + Add<Output = T> + Div<Output = T> + From<u32>,
    {
        if self.is_empty() {
            return None;
        }

        let first_idx = (self.head + self.buffer.len() as u32 - self.count) % self.buffer.len() as u32;
        let mut min = self.buffer[first_idx as usize].clone();
        let mut max = self.buffer[first_idx as usize].clone();
        let mut sum = self.buffer[first_idx as usize].clone();

        for i in 1..self.count {
            let idx = (self.head + self.buffer.len() as u32 - self.count + i) % self.buffer.len() as u32;
            let value = &self.buffer[idx as usize];
            if value < &min {
                min = value.clone();
            }
            if value > &max {
                max = value.clone();
            }
            sum = sum + value.clone();
        }

        let mean = sum / T::from(self.count);
        Some((min, max, mean))
    }
}
