use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use ros_z::time::Time;

use crate::{RetentionPolicy, SampleRecord};

pub struct TimeIndexedHistory<V> {
    entries: TimestampIndex<SampleRecord<V>>,
    policy: RetentionPolicy,
}

struct TimestampIndex<T> {
    entries: BTreeMap<Time, VecDeque<Arc<T>>>,
    len: usize,
}

impl<T> TimestampIndex<T> {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            len: 0,
        }
    }

    fn insert(&mut self, timestamp: Time, value: Arc<T>) {
        self.entries.entry(timestamp).or_default().push_back(value);
        self.len += 1;
    }

    fn get_interval(&self, start: Time, end: Time) -> Vec<Arc<T>> {
        if start > end {
            return Vec::new();
        }

        self.entries
            .range(start..=end)
            .flat_map(|(_, values)| values.iter().cloned())
            .collect()
    }

    fn latest_stamp(&self) -> Option<Time> {
        self.entries.keys().next_back().copied()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn remove_older_than(&mut self, minimum: Time) {
        let mut older = std::mem::take(&mut self.entries);
        let mut retained = older.split_off(&minimum);
        self.len = retained.values().map(VecDeque::len).sum();
        std::mem::swap(&mut self.entries, &mut retained);
    }

    fn pop_oldest(&mut self) -> Option<Arc<T>> {
        let oldest_timestamp = *self.entries.keys().next()?;
        let values = self.entries.get_mut(&oldest_timestamp)?;
        let value = values.pop_front()?;
        self.len -= 1;

        if values.is_empty() {
            self.entries.remove(&oldest_timestamp);
        }

        Some(value)
    }
}

impl<V> TimeIndexedHistory<V> {
    pub fn new(policy: RetentionPolicy) -> Self {
        Self {
            entries: TimestampIndex::new(),
            policy,
        }
    }

    pub fn insert(&mut self, record: Arc<SampleRecord<V>>) {
        let source_time = record.source_time;
        self.entries.insert(source_time, record);

        self.evict();
    }

    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.entries.get_interval(start, end)
    }

    fn evict(&mut self) {
        let Some(newest_source_time) = self.entries.latest_stamp() else {
            return;
        };

        let (duration, max_samples) = match self.policy {
            RetentionPolicy::LatestOnly => {
                self.entries.remove_older_than(newest_source_time);
                return;
            }
            RetentionPolicy::TimeWindow(window) => (window.duration(), window.max_samples()),
        };

        let minimum_source_time = newest_source_time.saturating_sub(duration);
        self.entries.remove_older_than(minimum_source_time);

        if let Some(max_samples) = max_samples {
            while self.entries.len() > max_samples.get() {
                self.entries.pop_oldest();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use ros_z::time::Time;

    use super::TimeIndexedHistory;
    use crate::{
        DEFAULT_TIME_WINDOW_MAX_SAMPLES, RetentionPolicy, SampleMetadata, SampleRecord,
        TopicSelector,
    };

    fn test_type_info() -> ros_z::TypeInfo {
        ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero())
    }

    fn test_publication_id() -> ros_z::pubsub::PublicationId {
        ros_z::pubsub::Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 1,
            source_global_id: ros_z::EndpointGlobalId::from([7; 16]),
        }
        .publication_id()
    }

    fn test_metadata() -> Arc<SampleMetadata> {
        Arc::new(SampleMetadata {
            requested_topic: TopicSelector::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            type_info: test_type_info(),
        })
    }

    fn record(value: i32, source_time_nanos: i64) -> Arc<SampleRecord<i32>> {
        Arc::new(SampleRecord {
            value,
            source_time: Time::from_nanos(source_time_nanos),
            transport_time: None,
            publication_id: test_publication_id(),
            metadata: test_metadata(),
        })
    }

    #[test]
    fn time_window_eviction_uses_source_time() {
        let mut history =
            TimeIndexedHistory::new(RetentionPolicy::time_window(Duration::from_secs(1)).unwrap());

        history.insert(record(1, 0));
        history.insert(record(2, 2_000_000_000));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(2_000_000_000));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn max_samples_evicts_oldest_after_time_eviction() {
        let mut history = TimeIndexedHistory::new(
            RetentionPolicy::time_window_with_max_samples(
                Duration::from_secs(10),
                std::num::NonZeroUsize::new(2).unwrap(),
            )
            .unwrap(),
        );

        history.insert(record(1, 1));
        history.insert(record(2, 2));
        history.insert(record(3, 3));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(10));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn duplicate_timestamps_preserve_insertion_order() {
        let mut history =
            TimeIndexedHistory::new(RetentionPolicy::time_window(Duration::from_secs(10)).unwrap());

        history.insert(record(1, 5));
        history.insert(record(2, 5));

        let values = history.window(Time::from_nanos(5), Time::from_nanos(5));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn latest_only_retains_only_latest_source_time_records() {
        let mut history = TimeIndexedHistory::new(RetentionPolicy::LatestOnly);

        history.insert(record(1, 1));
        history.insert(record(2, 3));
        history.insert(record(3, 2));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(3));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn latest_only_preserves_duplicate_newest_timestamp_order() {
        let mut history = TimeIndexedHistory::new(RetentionPolicy::LatestOnly);

        history.insert(record(1, 1));
        history.insert(record(2, 3));
        history.insert(record(3, 3));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(3));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![2, 3]
        );
    }

    #[test]
    fn time_window_eviction_uses_newest_source_time_for_out_of_order_inserts() {
        let mut history =
            TimeIndexedHistory::new(RetentionPolicy::time_window(Duration::from_secs(1)).unwrap());

        history.insert(record(1, 2_000_000_000));
        history.insert(record(2, 0));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(2_000_000_000));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn default_time_window_retention_bounds_stalled_source_timestamps() {
        let mut history =
            TimeIndexedHistory::new(RetentionPolicy::time_window(Duration::from_secs(10)).unwrap());

        for value in 0..=DEFAULT_TIME_WINDOW_MAX_SAMPLES {
            history.insert(record(value as i32, 5));
        }

        let values = history.window(Time::from_nanos(5), Time::from_nanos(5));
        assert_eq!(values.len(), DEFAULT_TIME_WINDOW_MAX_SAMPLES);
        assert_eq!(values.first().map(|record| record.value), Some(1));
        assert_eq!(
            values.last().map(|record| record.value),
            Some(DEFAULT_TIME_WINDOW_MAX_SAMPLES as i32)
        );
    }
}
