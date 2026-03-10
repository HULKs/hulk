//! Fixed-capacity timestamp-indexed cache.
//!
//! [`Cache`] is a low-level building block used by [`Buffer`](crate::Buffer).
//! Most users should use `Buffer` via [`Node::buffer()`](crate::Node::buffer).

use std::collections::{
    BTreeMap,
    Bound::{Included, Unbounded},
};

use crate::Timestamp;

/// A fixed-capacity cache indexed by timestamp.
///
/// Automatically evicts the oldest entries when capacity is exceeded.
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

    /// Returns the number of elements in the cache.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU128;
    use zenoh::time::{Timestamp as ZTimestamp, NTP64};

    fn make_timestamp(nanos: u64) -> Timestamp {
        let id = NonZeroU128::new(1).unwrap();
        ZTimestamp::new(NTP64(nanos), id.into())
    }

    #[test]
    fn new_cache_is_empty() {
        let cache: Cache<i32> = Cache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(cache.get_latest().is_none());
    }

    #[test]
    fn add_and_retrieve() {
        let mut cache = Cache::new(10);
        let ts = make_timestamp(1000);
        cache.add(ts, 42);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get_latest(), Some(&42));
    }

    #[test]
    fn capacity_enforced() {
        let mut cache = Cache::new(3);

        cache.add(make_timestamp(1), "first");
        cache.add(make_timestamp(2), "second");
        cache.add(make_timestamp(3), "third");
        assert_eq!(cache.len(), 3);

        // Adding a 4th should evict the oldest
        cache.add(make_timestamp(4), "fourth");
        assert_eq!(cache.len(), 3);

        // "first" should be gone
        assert_eq!(cache.get_oldest_time(), Some(&make_timestamp(2)));
        assert_eq!(cache.get_latest(), Some(&"fourth"));
    }

    #[test]
    fn get_elem_before_time() {
        let mut cache = Cache::new(10);
        cache.add(make_timestamp(100), "a");
        cache.add(make_timestamp(200), "b");
        cache.add(make_timestamp(300), "c");

        // Exact match
        assert_eq!(cache.get_elem_before_time(&make_timestamp(200)), Some(&"b"));

        // Between values - should get the one before
        assert_eq!(cache.get_elem_before_time(&make_timestamp(250)), Some(&"b"));

        // Before all - should be None
        assert_eq!(cache.get_elem_before_time(&make_timestamp(50)), None);
    }

    #[test]
    fn get_elem_after_time() {
        let mut cache = Cache::new(10);
        cache.add(make_timestamp(100), "a");
        cache.add(make_timestamp(200), "b");
        cache.add(make_timestamp(300), "c");

        // Exact match
        assert_eq!(cache.get_elem_after_time(&make_timestamp(200)), Some(&"b"));

        // Between values - should get the one after
        assert_eq!(cache.get_elem_after_time(&make_timestamp(150)), Some(&"b"));

        // After all - should be None
        assert_eq!(cache.get_elem_after_time(&make_timestamp(400)), None);
    }

    #[test]
    fn get_interval() {
        let mut cache = Cache::new(10);
        cache.add(make_timestamp(100), 1);
        cache.add(make_timestamp(200), 2);
        cache.add(make_timestamp(300), 3);
        cache.add(make_timestamp(400), 4);

        let interval = cache.get_interval(&make_timestamp(150), &make_timestamp(350));
        assert_eq!(interval, vec![&2, &3]);

        // Inclusive bounds
        let interval = cache.get_interval(&make_timestamp(200), &make_timestamp(300));
        assert_eq!(interval, vec![&2, &3]);
    }

    #[test]
    fn oldest_and_latest_time() {
        let mut cache = Cache::new(10);
        assert!(cache.get_oldest_time().is_none());
        assert!(cache.get_latest_time().is_none());

        cache.add(make_timestamp(500), "x");
        cache.add(make_timestamp(100), "y");
        cache.add(make_timestamp(300), "z");

        assert_eq!(cache.get_oldest_time(), Some(&make_timestamp(100)));
        assert_eq!(cache.get_latest_time(), Some(&make_timestamp(500)));
    }
}
