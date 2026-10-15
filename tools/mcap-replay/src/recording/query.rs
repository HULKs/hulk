use std::{
    slice::from_ref,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use color_eyre::eyre::{Result, ensure, eyre};
use ros_z::time::Time;
use ros_z_debug::query::{
    Coverage, MAX_PAYLOAD_BYTES, RawRecord, SourceCoverage, TopicInfo,
    plan::{Evaluation, LoadedRecord, QueryPlan, SourceState, SourceView, State},
    remote::RecordingInfo,
};
use uuid::Uuid;

use super::{Recording, SnapshotChunk};
use index::{INDEX_WORKERS, Index, IndexCache, IndexEntry, IndexKey};

mod index;
#[cfg(test)]
pub(super) mod tests;

pub const DEFAULT_INDEX_BYTES: usize = 256 * 1024 * 1024;

/// Coordinates progressive queries; each evaluation pins its acquired indexes.
pub struct QueryRecording {
    cancel: Arc<AtomicBool>,
    recording: Recording,
    publication_publisher: [u8; 16],
    cache: IndexCache,
    index_workers: usize,
    pub info: RecordingInfo,
}

impl QueryRecording {
    pub fn new(recording: Recording, name: String, cancel: Arc<AtomicBool>) -> Result<Self> {
        let instance = Uuid::new_v4();
        let info = RecordingInfo {
            instance: instance.to_string(),
            name,
            // Summary log-time bounds require no scan; query indexes refine them.
            start: Time::from_nanos(recording.start.try_into()?),
            end: Time::from_nanos(recording.end.try_into()?),
            topics: recording
                .channels
                .values()
                .map(|channel| TopicInfo {
                    name: channel.topic.clone(),
                    type_info: channel.type_info.clone(),
                    schema: Some(Arc::clone(&channel.schema)),
                    error: None,
                })
                .collect(),
        };
        Ok(Self {
            cancel,
            recording,
            publication_publisher: *instance.as_bytes(),
            cache: IndexCache::new(DEFAULT_INDEX_BYTES),
            index_workers: INDEX_WORKERS,
            info,
        })
    }

    pub fn with_index_budget(mut self, bytes: usize) -> Result<Self> {
        self.cache.set_budget(bytes)?;
        Ok(self)
    }

    fn channel(&self, topic: &str) -> Result<u16> {
        self.recording
            .channels
            .iter()
            .find(|(_, channel)| channel.topic == topic)
            .map(|(id, _)| *id)
            .ok_or_else(|| eyre!("recording has no topic {topic}"))
    }

    fn source_view(
        &mut self,
        key: Option<&IndexKey>,
        pinned: &mut Option<Arc<Index>>,
        failure: Option<&String>,
    ) -> SourceState {
        let Some(key) = key else {
            return SourceState::Absent;
        };
        if let Some(error) = failure {
            return SourceState::Failed(error.clone());
        }
        if pinned.is_none() {
            *pinned = self.cache.get(key);
        }
        let Some(index) = pinned else {
            return SourceState::Pending;
        };
        let channel = &self.recording.channels[&key.0];
        SourceState::Ready(SourceView {
            topic: TopicInfo {
                name: channel.topic.clone(),
                type_info: channel.type_info.clone(),
                schema: None,
                error: None,
            },
            index: index.clone(),
            coverage: Coverage {
                source: SourceCoverage::Complete,
                truncated: false,
            },
            diagnostics: index.diagnostics.clone(),
        })
    }

    fn record(
        &self,
        channel: u16,
        entry: IndexEntry,
        chunk: &SnapshotChunk,
    ) -> Result<Arc<RawRecord>> {
        let (header, data) = chunk.message(entry.record_offset)?;
        ensure!(
            header.channel_id == channel,
            "indexed message channel mismatch"
        );
        let id = (u64::from(entry.chunk_id) << 32) | u64::from(entry.record_offset);
        Ok(Arc::new(RawRecord {
            // Reopening a file changes its publisher; the cast preserves all location bits.
            publication: Some((self.publication_publisher, id as i64)),
            id,
            time: entry.time(),
            source_time: Time::from_nanos(header.publish_time.try_into()?),
            log_time: Some(Time::from_nanos(header.log_time.try_into()?)),
            data: data.as_ref().into(),
        }))
    }

    fn load_records(
        &mut self,
        keys: &[Option<IndexKey>],
        pinned: &[Option<Arc<Index>>],
        locations: &[(usize, usize)],
    ) -> Vec<LoadedRecord> {
        let mut ordered: Vec<_> = locations
            .iter()
            .enumerate()
            .map(|(output, &(source, slot))| {
                let entry = pinned[source].as_ref().expect("pinned source").entries[slot];
                (entry.chunk_id, source, slot, output)
            })
            .collect();
        ordered.sort_unstable();
        let mut results = vec![None; locations.len()];
        let mut chunk = None;
        let mut bytes = 0;
        for (chunk_id, source, slot, output) in ordered {
            if bytes >= MAX_PAYLOAD_BYTES as usize {
                results[output] = Some(Err("plan payload exceeds 32 MiB".into()));
                continue;
            }
            if chunk.as_ref().is_none_or(|(id, _)| *id != chunk_id) {
                chunk = Some((
                    chunk_id,
                    self.recording
                        .snapshot_chunk(chunk_id, &self.cancel)
                        .map_err(|error| format!("{error:#}")),
                ));
            }
            let key = keys[source].as_ref().expect("ready source key");
            let entry = pinned[source].as_ref().expect("pinned source").entries[slot];
            let loaded = match &chunk.as_ref().expect("loaded chunk").1 {
                Ok(chunk) => self
                    .record(key.0, entry, chunk)
                    .map_err(|error| format!("{error:#}")),
                Err(error) => Err(error.clone()),
            };
            if let Ok(record) = &loaded {
                bytes += record.data.len();
            }
            results[output] = Some(if bytes > MAX_PAYLOAD_BYTES as usize {
                Err("plan payload exceeds 32 MiB".into())
            } else {
                loaded
            });
        }
        results
            .into_iter()
            .map(|record| record.expect("visited payload location"))
            .collect()
    }

    /// Emit primary results before building optional indexes; failures stay local.
    pub fn query_plan(
        &mut self,
        plan: &QueryPlan,
        cursor: Time,
        mut emit: impl FnMut(Evaluation) -> bool,
    ) -> Result<()> {
        let sources = plan.sources()?;
        let keys: Vec<_> = sources
            .iter()
            .map(|source| {
                self.channel(&source.topic)
                    .ok()
                    .map(|channel| (channel, source.projection.clone()))
            })
            .collect();
        let mut pinned = vec![None; sources.len()];
        let mut failures: Vec<Option<String>> = vec![None; sources.len()];
        loop {
            ensure!(!self.cancel.load(Ordering::Relaxed), "query cancelled");
            let views: Vec<_> = keys
                .iter()
                .zip(&mut pinned)
                .zip(&failures)
                .map(|((key, pinned), failure)| {
                    self.source_view(key.as_ref(), pinned, failure.as_ref())
                })
                .collect();
            let result = plan.evaluate_batched(cursor, &views, |locations| {
                self.load_records(&keys, &pinned, locations)
            })?;
            let pending = result
                .dependencies
                .iter()
                .find(|dependency| dependency.resolution == State::Pending)
                .and_then(|dependency| {
                    sources
                        .iter()
                        .position(|source| source == &dependency.source)
                });
            drop(views);
            for (source, pin) in sources.iter().zip(&mut pinned) {
                if !result
                    .dependencies
                    .iter()
                    .any(|dependency| &dependency.source == source)
                {
                    *pin = None;
                }
            }
            if !emit(result) {
                return Ok(());
            }
            let Some(index) = pending else { return Ok(()) };
            let key = keys[index].as_ref().expect("pending source key");
            match self.cache.indexes(
                &self.recording,
                &self.cancel,
                self.index_workers,
                from_ref(key),
            ) {
                Ok(mut indexes) => pinned[index] = indexes.pop(),
                Err(error) => failures[index] = Some(format!("{error:#}")),
            }
        }
    }
}
