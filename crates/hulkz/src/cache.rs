use std::{
    collections::{
        BTreeMap,
        Bound::{Included, Unbounded},
    },
    time::Duration,
};

use crate::Timestamped;

pub struct Cache<T> {
    buffer: BTreeMap<Duration, T>,
    capacity: usize,
}

impl<T> Cache<T>
where
    T: Timestamped + Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: BTreeMap::new(),
            capacity,
        }
    }

    /// Add a message to the cache.
    /// Prunes the oldest messages if capacity is exceeded.
    pub fn add(&mut self, msg: T) {
        let time = msg.timestamp();
        self.buffer.insert(time, msg);
        while self.buffer.len() > self.capacity {
            self.buffer.pop_first();
        }
    }

    /// Return the oldest element after or equal to the passed time stamp.
    pub fn get_elem_after_time(&self, stamp: Duration) -> Option<T> {
        self.buffer
            .range((Included(stamp), Unbounded))
            .next()
            .map(|(_time, msg)| msg.clone())
    }

    /// Return the newest element before or equal to the passed time stamp.
    pub fn get_elem_before_time(&self, stamp: Duration) -> Option<T> {
        self.buffer
            .range((Unbounded, Included(stamp)))
            .next_back()
            .map(|(_time, msg)| msg.clone())
    }

    /// Query the current cache content between from_stamp and to_stamp.
    pub fn get_interval(&self, start: Duration, end: Duration) -> Vec<T> {
        self.buffer
            .range((Included(start), Included(end)))
            .map(|(_time, msg)| msg.clone())
            .collect()
    }

    /// Return the newest recorded message.
    pub fn get_latest(&self) -> Option<T> {
        self.buffer.last_key_value().map(|(_, msg)| msg.clone())
    }

    /// Return the timestamp of the newest message.
    pub fn get_latest_time(&self) -> Option<Duration> {
        self.buffer.keys().last().copied()
    }

    /// Return the timestamp of the oldest message.
    pub fn get_oldest_time(&self) -> Option<Duration> {
        self.buffer.keys().next().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: implement unit tests for the cache
}
