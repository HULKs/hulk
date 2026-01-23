use std::collections::{
    BTreeMap,
    Bound::{Included, Unbounded},
};

use crate::Timestamp;

pub struct Cache<T> {
    buffer: BTreeMap<Timestamp, T>,
    capacity: usize,
}

impl<T> Cache<T>
where
    T: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: BTreeMap::new(),
            capacity,
        }
    }

    /// Add a message to the cache.
    /// Prunes the oldest messages if capacity is exceeded.
    pub fn add(&mut self, timestamp: Timestamp, value: T) {
        self.buffer.insert(timestamp, value);
        while self.buffer.len() > self.capacity {
            self.buffer.pop_first();
        }
    }

    /// Return the oldest element after or equal to the passed time stamp.
    pub fn get_elem_after_time(&self, timestamp: &Timestamp) -> Option<&T> {
        self.buffer
            .range((Included(timestamp), Unbounded))
            .next()
            .map(|(_time, msg)| msg)
    }

    /// Return the newest element before or equal to the passed time stamp.
    pub fn get_elem_before_time(&self, timestamp: &Timestamp) -> Option<&T> {
        self.buffer
            .range((Unbounded, Included(timestamp)))
            .next_back()
            .map(|(_time, msg)| msg)
    }

    /// Query the current cache content between from_stamp and to_stamp.
    pub fn get_interval(&self, start: &Timestamp, end: &Timestamp) -> Vec<&T> {
        self.buffer
            .range((Included(start), Included(end)))
            .map(|(_time, msg)| msg)
            .collect()
    }

    /// Return the newest recorded message.
    pub fn get_latest(&self) -> Option<&T> {
        self.buffer.last_key_value().map(|(_time, msg)| msg)
    }

    /// Return the timestamp of the newest message.
    pub fn get_latest_time(&self) -> Option<&Timestamp> {
        self.buffer.keys().last()
    }

    /// Return the timestamp of the oldest message.
    pub fn get_oldest_time(&self) -> Option<&Timestamp> {
        self.buffer.keys().next()
    }
}
