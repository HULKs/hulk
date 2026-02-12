use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use hulkz::Timestamp;

use crate::{
    keyspace::source_key,
    types::{CacheStats, SourceSpec, StreamRecord},
};

type SourceTimestampBucket = BTreeMap<Timestamp, Vec<(u64, Arc<StreamRecord>)>>;

#[derive(Debug, Clone)]
struct CacheEntry {
    source_key: String,
    timestamp: Timestamp,
    seq: u64,
    size: usize,
}

#[derive(Debug)]
pub struct GlobalCache {
    budget_bytes: usize,
    bytes_used: usize,
    next_seq: u64,
    entries_by_order: BTreeMap<(Timestamp, u64), CacheEntry>,
    by_source: HashMap<String, SourceTimestampBucket>,
    scrub_window: Option<(Timestamp, Timestamp)>,
    hit_count: u64,
    miss_count: u64,
    eviction_count: u64,
}

impl GlobalCache {
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            budget_bytes,
            bytes_used: 0,
            next_seq: 0,
            entries_by_order: BTreeMap::new(),
            by_source: HashMap::new(),
            scrub_window: None,
            hit_count: 0,
            miss_count: 0,
            eviction_count: 0,
        }
    }

    /// Defines a scrub working-set window that is preferentially kept in RAM.
    pub fn set_scrub_window(&mut self, window: Option<(Timestamp, Timestamp)>) {
        self.scrub_window = window;
        self.evict_if_needed();
    }

    pub fn insert(&mut self, record: Arc<StreamRecord>) {
        let source = source_key(&record.source);
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let size = record.payload.len();

        let entry = CacheEntry {
            source_key: source.clone(),
            timestamp: record.timestamp,
            seq,
            size,
        };

        self.entries_by_order.insert((entry.timestamp, seq), entry);
        self.by_source
            .entry(source)
            .or_default()
            .entry(record.timestamp)
            .or_default()
            .push((seq, record));
        self.bytes_used = self.bytes_used.saturating_add(size);

        self.evict_if_needed();
    }

    pub fn latest(&mut self, spec: &SourceSpec) -> Option<Arc<StreamRecord>> {
        let key = source_key(spec);
        let found = self
            .by_source
            .get(&key)
            .and_then(|records| records.last_key_value())
            .and_then(|(_ts, bucket)| bucket.last())
            .map(|(_seq, record)| record.clone());
        self.bump_lookup_stats(found.is_some());
        found
    }

    pub fn before_or_equal(
        &mut self,
        spec: &SourceSpec,
        timestamp: Timestamp,
    ) -> Option<Arc<StreamRecord>> {
        let key = source_key(spec);
        let found = self
            .by_source
            .get(&key)
            .and_then(|records| records.range(..=timestamp).next_back())
            .and_then(|(_ts, bucket)| bucket.last())
            .map(|(_seq, record)| record.clone());
        self.bump_lookup_stats(found.is_some());
        found
    }

    pub fn nearest(
        &mut self,
        spec: &SourceSpec,
        timestamp: Timestamp,
    ) -> Option<Arc<StreamRecord>> {
        let key = source_key(spec);
        let found = self.by_source.get(&key).and_then(|records| {
            let before = records
                .range(..=timestamp)
                .next_back()
                .and_then(|(_ts, bucket)| bucket.last())
                .map(|(_seq, record)| record.clone());
            let after = records
                .range(timestamp..)
                .next()
                .and_then(|(_ts, bucket)| bucket.first())
                .map(|(_seq, record)| record.clone());

            match (before, after) {
                (Some(before), Some(after)) => {
                    let diff_before = timestamp
                        .get_time()
                        .to_duration()
                        .abs_diff(before.timestamp.get_time().to_duration());
                    let diff_after = timestamp
                        .get_time()
                        .to_duration()
                        .abs_diff(after.timestamp.get_time().to_duration());
                    if diff_before <= diff_after {
                        Some(before)
                    } else {
                        Some(after)
                    }
                }
                (Some(record), None) | (None, Some(record)) => Some(record),
                (None, None) => None,
            }
        });

        self.bump_lookup_stats(found.is_some());
        found
    }

    pub fn range_inclusive(
        &mut self,
        spec: &SourceSpec,
        start: Timestamp,
        end: Timestamp,
    ) -> Vec<Arc<StreamRecord>> {
        let key = source_key(spec);
        let records = self
            .by_source
            .get(&key)
            .map(|source_records| {
                source_records
                    .range(start..=end)
                    .flat_map(|(_ts, bucket)| bucket.iter().map(|(_seq, record)| record.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        self.bump_lookup_stats(!records.is_empty());
        records
    }

    pub fn range_inclusive_all(&self, start: Timestamp, end: Timestamp) -> Vec<Arc<StreamRecord>> {
        let mut records = Vec::new();
        for source_records in self.by_source.values() {
            records.extend(
                source_records
                    .range(start..=end)
                    .flat_map(|(_ts, bucket)| bucket.iter().map(|(_seq, record)| record.clone())),
            );
        }
        records.sort_by_key(|record| record.timestamp);
        records
    }

    pub fn range_inclusive_after(
        &self,
        spec: &SourceSpec,
        start: Timestamp,
        end: Timestamp,
        strict_after: Option<Timestamp>,
    ) -> Vec<Arc<StreamRecord>> {
        let key = source_key(spec);
        self.by_source
            .get(&key)
            .map(|source_records| {
                source_records
                    .range(start..=end)
                    .flat_map(|(_ts, bucket)| bucket.iter().map(|(_seq, record)| record.clone()))
                    .filter(|record| match strict_after {
                        Some(frontier) => record.timestamp > frontier,
                        None => true,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            bytes_used: self.bytes_used,
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            eviction_count: self.eviction_count,
        }
    }

    pub fn latest_timestamp(&self) -> Option<Timestamp> {
        self.entries_by_order
            .last_key_value()
            .map(|((ts, _seq), _)| *ts)
    }

    fn bump_lookup_stats(&mut self, hit: bool) {
        if hit {
            self.hit_count = self.hit_count.saturating_add(1);
        } else {
            self.miss_count = self.miss_count.saturating_add(1);
        }
    }

    fn evict_if_needed(&mut self) {
        while self.bytes_used > self.budget_bytes {
            let eviction_key = self
                .oldest_outside_scrub_window()
                .or_else(|| self.entries_by_order.first_key_value().map(|(key, _)| *key));

            let Some(key) = eviction_key else {
                break;
            };

            if let Some(entry) = self.entries_by_order.remove(&key) {
                self.remove_from_source(&entry);
                self.bytes_used = self.bytes_used.saturating_sub(entry.size);
                self.eviction_count = self.eviction_count.saturating_add(1);
            } else {
                break;
            }
        }
    }

    fn oldest_outside_scrub_window(&self) -> Option<(Timestamp, u64)> {
        let Some((start, end)) = self.scrub_window else {
            return self.entries_by_order.first_key_value().map(|(key, _)| *key);
        };

        self.entries_by_order.iter().find_map(|(key, entry)| {
            if entry.timestamp < start || entry.timestamp > end {
                Some(*key)
            } else {
                None
            }
        })
    }

    fn remove_from_source(&mut self, entry: &CacheEntry) {
        let mut remove_source = false;

        if let Some(source_records) = self.by_source.get_mut(&entry.source_key) {
            if let Some(bucket) = source_records.get_mut(&entry.timestamp) {
                bucket.retain(|(seq, _record)| *seq != entry.seq);
                if bucket.is_empty() {
                    source_records.remove(&entry.timestamp);
                }
            }
            remove_source = source_records.is_empty();
        }

        if remove_source {
            self.by_source.remove(&entry.source_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hulkz::{Scope, ScopedPath};
    use zenoh::bytes::Encoding;

    use crate::{
        keyspace::from_nanos_with_id,
        types::{NamespaceBinding, PlaneKind, SourceSpec, StreamRecord},
    };

    use super::GlobalCache;

    fn make_ts(nanos: u64) -> hulkz::Timestamp {
        from_nanos_with_id(nanos, None)
    }

    fn spec(name: &str) -> SourceSpec {
        SourceSpec {
            plane: PlaneKind::Data,
            path: ScopedPath::new(Scope::Local, name),
            node_override: None,
            namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
        }
    }

    fn rec(source: &SourceSpec, ts: u64, payload_len: usize) -> Arc<StreamRecord> {
        Arc::new(StreamRecord {
            source: source.clone(),
            effective_namespace: Some("robot".to_string()),
            timestamp: make_ts(ts),
            encoding: Encoding::APPLICATION_CDR,
            payload: vec![0_u8; payload_len].into(),
        })
    }

    #[test]
    fn evicts_oldest_timestamp_first() {
        let mut cache = GlobalCache::new(6);
        let source = spec("x");

        cache.insert(rec(&source, 10, 3));
        cache.insert(rec(&source, 20, 3));
        cache.insert(rec(&source, 5, 3));

        let latest = cache.latest(&source).unwrap();
        assert_eq!(latest.timestamp, make_ts(20));

        let range = cache.range_inclusive(&source, make_ts(0), make_ts(30));
        assert_eq!(range.len(), 2);
        assert!(range.iter().all(|r| r.timestamp != make_ts(5)));
    }

    #[test]
    fn nearest_prefers_earlier_on_tie() {
        let mut cache = GlobalCache::new(100);
        let source = spec("x");

        cache.insert(rec(&source, 10, 1));
        cache.insert(rec(&source, 20, 1));

        let nearest = cache.nearest(&source, make_ts(15)).unwrap();
        assert_eq!(nearest.timestamp, make_ts(10));
    }

    #[test]
    fn scrub_window_protects_recent_scrubbed_data() {
        let mut cache = GlobalCache::new(6);
        let source = spec("x");

        cache.insert(rec(&source, 10, 3));
        cache.insert(rec(&source, 20, 3));

        cache.set_scrub_window(Some((make_ts(8), make_ts(12))));
        cache.insert(rec(&source, 30, 3));

        let kept = cache.range_inclusive(&source, make_ts(0), make_ts(40));
        assert!(kept.iter().any(|record| record.timestamp == make_ts(10)));
        assert!(!kept.iter().any(|record| record.timestamp == make_ts(20)));
    }
}
