use std::sync::Arc;

use color_eyre::eyre::{OptionExt, Result, ensure};
use ros_z::time::Time;
use ros_z_debug::query::{
    Coverage, MAX_PAYLOAD_BYTES, RawRecord, SourceCoverage, TopicInfo,
    plan::{Evaluation, LoadedRecord, QueryPlan, SourceState, SourceView, State, TopicSource},
    remote::RecordingInfo,
};
use tokio::sync::OwnedMutexGuard;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{Recording, SnapshotChunk};
pub use index::IndexCache;
use index::{INDEX_WORKERS, Index, IndexEntry, IndexKey};

mod index;
#[cfg(test)]
pub(super) mod tests;

/// Coordinates progressive queries; each evaluation pins its acquired indexes.
pub struct QueryRecording {
    cancel: CancellationToken,
    recording: Recording,
    publication_publisher: [u8; 16],
    cache: Arc<IndexCache>,
    owner: Uuid,
    index_workers: usize,
    pub info: RecordingInfo,
}

pub struct PendingQuery {
    plan: QueryPlan,
    cursor: Time,
    sources: Vec<TopicSource>,
    keys: Vec<Option<IndexKey>>,
    pinned: Vec<Option<Arc<Index>>>,
    failures: Vec<Option<String>>,
}

impl QueryRecording {
    pub fn new(
        recording: Recording,
        name: String,
        cancel: CancellationToken,
        cache: Arc<IndexCache>,
    ) -> Result<Self> {
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
            cache,
            owner: instance,
            index_workers: INDEX_WORKERS,
            info,
        })
    }

    fn channel(&self, topic: &str) -> Result<u16> {
        self.recording
            .channels
            .iter()
            .find(|(_, channel)| channel.topic == topic)
            .map(|(id, _)| *id)
            .ok_or_eyre(format!("recording has no topic {topic}"))
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
            *pinned = self.cache.get(&self.owner, key);
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
            if let Err(error) = &loaded {
                tracing::warn!(recording = %self.info.name, %error, "reading query payload failed");
            }
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

    pub async fn acquire_build(&self) -> OwnedMutexGuard<()> {
        self.cache.acquire_build().await
    }
}

impl PendingQuery {
    pub fn new(recording: &QueryRecording, plan: QueryPlan, cursor: Time) -> Result<Self> {
        let sources = plan.sources()?;
        let keys: Vec<_> = sources
            .iter()
            .map(|source| {
                recording
                    .channel(&source.topic)
                    .ok()
                    .map(|channel| (channel, source.projection.clone()))
            })
            .collect();
        Ok(Self {
            pinned: vec![None; sources.len()],
            failures: vec![None; sources.len()],
            plan,
            cursor,
            sources,
            keys,
        })
    }

    pub fn build(
        &mut self,
        recording: &QueryRecording,
        source: usize,
        guard: OwnedMutexGuard<()>,
    ) -> Result<()> {
        let key = self
            .keys
            .get(source)
            .and_then(Option::as_ref)
            .ok_or_eyre("missing pending source")?;
        match recording.cache.index(
            &recording.owner,
            &recording.recording,
            &recording.cancel,
            recording.index_workers,
            key,
            guard,
        ) {
            Ok(index) => self.pinned[source] = Some(index),
            Err(error) => {
                tracing::warn!(recording = %recording.info.name, error = %format!("{error:#}"), "building query index failed");
                self.failures[source] = Some(format!("{error:#}"));
            }
        }
        Ok(())
    }

    /// Evaluate cached inputs before asking the async driver for the build lock.
    pub fn advance(
        &mut self,
        recording: &mut QueryRecording,
        mut emit: impl FnMut(Evaluation) -> bool,
    ) -> Result<Option<usize>> {
        ensure!(!recording.cancel.is_cancelled(), "query cancelled");
        let views: Vec<_> = self
            .keys
            .iter()
            .zip(&mut self.pinned)
            .zip(&self.failures)
            .map(|((key, pinned), failure)| {
                recording.source_view(key.as_ref(), pinned, failure.as_ref())
            })
            .collect();
        let result = self
            .plan
            .evaluate_batched(self.cursor, &views, |locations| {
                recording.load_records(&self.keys, &self.pinned, locations)
            })?;
        let pending = result
            .dependencies
            .iter()
            .find(|dependency| dependency.resolution == State::Pending)
            .and_then(|dependency| {
                self.sources
                    .iter()
                    .position(|source| source == &dependency.source)
            });
        drop(views);
        for (source, pin) in self.sources.iter().zip(&mut self.pinned) {
            if !result
                .dependencies
                .iter()
                .any(|dependency| &dependency.source == source)
            {
                *pin = None;
            }
        }
        if !emit(result) {
            return Ok(None);
        }
        Ok(pending)
    }
}

impl Drop for QueryRecording {
    fn drop(&mut self) {
        self.cache.remove_recording(self.owner);
    }
}
