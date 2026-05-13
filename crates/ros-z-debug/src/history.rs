use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

use ros_z::time::Time;

use crate::{RetentionPolicy, SampleRecord};

pub struct TimeIndexedHistory<V> {
    entries: BTreeMap<Time, VecDeque<Arc<SampleRecord<V>>>>,
    policy: RetentionPolicy,
    len: usize,
}

impl<V> TimeIndexedHistory<V> {
    pub fn new(policy: RetentionPolicy) -> Self {
        Self {
            entries: BTreeMap::new(),
            policy,
            len: 0,
        }
    }

    pub fn insert(&mut self, record: Arc<SampleRecord<V>>) {
        let source_time = record.source_time;
        self.entries
            .entry(source_time)
            .or_default()
            .push_back(record);
        self.len += 1;

        self.evict();
    }

    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<SampleRecord<V>>> {
        self.entries
            .range(start..=end)
            .flat_map(|(_, records)| records.iter().cloned())
            .collect()
    }

    fn evict(&mut self) {
        let Some(newest_source_time) = self
            .entries
            .last_key_value()
            .map(|(source_time, _)| *source_time)
        else {
            return;
        };

        let (duration, max_samples) = match self.policy {
            RetentionPolicy::LatestOnly => {
                self.remove_entries_older_than(newest_source_time);
                return;
            }
            RetentionPolicy::TimeWindow {
                duration,
                max_samples,
            } => (duration, max_samples),
        };

        let minimum_source_time = newest_source_time.saturating_sub(duration);
        self.remove_entries_older_than(minimum_source_time);

        if let Some(max_samples) = max_samples {
            while self.len > max_samples {
                self.pop_oldest();
            }
        }
    }

    fn remove_entries_older_than(&mut self, minimum_source_time: Time) {
        while self
            .entries
            .first_key_value()
            .is_some_and(|(source_time, _)| *source_time < minimum_source_time)
        {
            self.pop_oldest();
        }
    }

    fn pop_oldest(&mut self) {
        let Some(source_time) = self
            .entries
            .first_key_value()
            .map(|(source_time, _)| *source_time)
        else {
            return;
        };

        let should_remove_entry = self.entries.get_mut(&source_time).is_some_and(|records| {
            let removed = records.pop_front();
            if removed.is_some() {
                self.len -= 1;
            }
            records.is_empty()
        });

        if should_remove_entry {
            self.entries.remove(&source_time);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use ros_z::time::Time;

    use super::TimeIndexedHistory;
    use crate::{RetentionPolicy, SampleRecord, TopicSelector};

    fn record(value: i32, source_time_nanos: i64) -> Arc<SampleRecord<i32>> {
        Arc::new(SampleRecord {
            value,
            source_time: Time::from_nanos(source_time_nanos),
            transport_time: None,
            publication_id: None,
            source_global_id: None,
            requested_topic: TopicSelector::new("debug").unwrap(),
            resolved_topic: "/debug".to_string(),
            namespace_version: 0,
            type_info: None,
            schema: None,
        })
    }

    #[test]
    fn time_window_eviction_uses_source_time() {
        let mut history = TimeIndexedHistory::new(RetentionPolicy::TimeWindow {
            duration: Duration::from_secs(1),
            max_samples: None,
        });

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
        let mut history = TimeIndexedHistory::new(RetentionPolicy::TimeWindow {
            duration: Duration::from_secs(10),
            max_samples: Some(2),
        });

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
        let mut history = TimeIndexedHistory::new(RetentionPolicy::TimeWindow {
            duration: Duration::from_secs(10),
            max_samples: None,
        });

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
        let mut history = TimeIndexedHistory::new(RetentionPolicy::TimeWindow {
            duration: Duration::from_secs(1),
            max_samples: None,
        });

        history.insert(record(1, 2_000_000_000));
        history.insert(record(2, 0));

        let values = history.window(Time::from_nanos(0), Time::from_nanos(2_000_000_000));
        assert_eq!(
            values.iter().map(|record| record.value).collect::<Vec<_>>(),
            vec![1]
        );
    }
}
