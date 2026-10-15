use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    slice::from_ref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use color_eyre::eyre::{Context, Result, bail, ensure, eyre};
use ros_z::{dynamic::Schema, time::Time};
#[cfg(test)]
use ros_z_debug::query::plan::TopicSource;
use ros_z_debug::query::{
    Coverage, MAX_PAYLOAD_BYTES, RawRecord, SourceCoverage, TimeProjection, TopicInfo,
    plan::{Evaluation, QueryPlan, TimeIndex},
    remote::RecordingInfo,
};
use uuid::Uuid;

use super::{Recording, SnapshotChunk};

pub const DEFAULT_INDEX_BYTES: usize = 256 * 1024 * 1024;
const MAX_INDEXES: usize = 256;
const ENTRY_BYTES: usize = size_of::<IndexEntry>();
const INDEX_WORKERS: usize = 4;
const INDEX_BATCH_RECORDS: usize = 1024;

/// File location also breaks timestamp ties, without a separate ordering field.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IndexEntry {
    timestamp_ns: i64,
    chunk_id: u32,
    record_offset: u32,
}

const _: () = assert!(ENTRY_BYTES == 16);

impl IndexEntry {
    fn time(&self) -> Time {
        Time::from_nanos(self.timestamp_ns)
    }
}

#[derive(Default)]
struct Index {
    entries: Vec<IndexEntry>,
    diagnostics: Vec<String>,
}

impl Index {
    fn bytes(&self) -> usize {
        self.entries.capacity() * ENTRY_BYTES
    }
}

impl TimeIndex for Index {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn time(&self, index: usize) -> Time {
        self.entries[index].time()
    }
}

type IndexKey = (u16, TimeProjection);

struct CachedIndex {
    index: Arc<Index>,
    last_used: u64,
}

struct IndexCache {
    entries: HashMap<IndexKey, CachedIndex>,
    bytes: usize,
    budget: usize,
    clock: u64,
}

impl IndexCache {
    fn get(&mut self, key: &IndexKey) -> Option<Arc<Index>> {
        let cached = self.entries.get_mut(key)?;
        self.clock = self.clock.saturating_add(1);
        cached.last_used = self.clock;
        Some(Arc::clone(&cached.index))
    }

    /// Callers pin query inputs by holding Arcs. Builders are charged separately
    /// until the whole batch succeeds, so errors/cancellation cannot leak budget.
    fn make_room(&mut self, building_bytes: usize, building_count: usize) -> Result<()> {
        while self.bytes.saturating_add(building_bytes) > self.budget
            || self.entries.len() + building_count > MAX_INDEXES
        {
            let oldest = self
                .entries
                .iter()
                .filter(|(_, cached)| Arc::strong_count(&cached.index) == 1)
                .min_by_key(|(_, cached)| cached.last_used)
                .map(|(key, _)| key.clone());
            let Some(key) = oldest else {
                bail!(
                    "indexes required by this query exceed the {} byte cache budget or {MAX_INDEXES}-index limit; increase --index-cache-mib or reduce the query",
                    self.budget
                );
            };
            if let Some(cached) = self.entries.remove(&key) {
                self.bytes -= cached.index.bytes();
            }
        }
        Ok(())
    }

    fn reserve_entry(
        &mut self,
        entries: &mut Vec<IndexEntry>,
        hint: usize,
        building_bytes: &mut usize,
        building_count: usize,
    ) -> Result<()> {
        if entries.len() < entries.capacity() {
            return Ok(());
        }
        let pinned: usize = self
            .entries
            .values()
            .filter(|cached| Arc::strong_count(&cached.index) > 1)
            .map(|cached| cached.index.bytes())
            .sum();
        let available = self
            .budget
            .saturating_sub(pinned)
            .saturating_sub(*building_bytes)
            / ENTRY_BYTES;
        ensure!(
            available > 0,
            "indexes required by this query exceed the {} byte cache budget; increase --index-cache-mib or reduce the query",
            self.budget
        );
        let growth = if entries.capacity() == 0 {
            if hint > 0 && hint <= available {
                hint
            } else {
                256.min(available)
            }
        } else {
            entries.capacity().min(available)
        };
        self.make_room(
            building_bytes.saturating_add(growth * ENTRY_BYTES),
            building_count,
        )?;
        let previous = entries.capacity();
        entries.try_reserve_exact(growth)?;
        *building_bytes += (entries.capacity() - previous) * ENTRY_BYTES;
        // Account for actual capacity, including any allocator over-reservation.
        self.make_room(*building_bytes, building_count)
    }

    fn insert(&mut self, key: IndexKey, index: Arc<Index>) {
        self.clock = self.clock.saturating_add(1);
        self.bytes += index.bytes();
        let previous = self.entries.insert(
            key,
            CachedIndex {
                index,
                last_used: self.clock,
            },
        );
        debug_assert!(previous.is_none());
        debug_assert!(self.bytes <= self.budget);
    }
}

struct PendingIndex {
    key: IndexKey,
    schema: Schema,
    index: Index,
    hint: usize,
    rejected: usize,
    last_error: Option<((u32, u32), String)>,
}

enum IndexedRecord {
    Timestamp {
        index: usize,
        entry: IndexEntry,
    },
    Rejected {
        index: usize,
        location: (u32, u32),
        error: String,
    },
}

/// Workers stage at most one small batch each. The single build state owns all
/// index allocation/eviction; no lock is held while reading or projecting data.
struct IndexBuild<'a> {
    pending: &'a mut [PendingIndex],
    cache: &'a mut IndexCache,
    building_bytes: usize,
    #[cfg(test)]
    decoded_chunks: usize,
    #[cfg(test)]
    projected_messages: usize,
}

impl IndexBuild<'_> {
    fn merge(&mut self, batch: &mut Vec<IndexedRecord>) -> Result<()> {
        let count = self.pending.len();
        for record in batch.drain(..) {
            let index = match &record {
                IndexedRecord::Timestamp { index, .. } | IndexedRecord::Rejected { index, .. } => {
                    *index
                }
            };
            let pending = &mut self.pending[index];
            #[cfg(test)]
            if pending.key.1 != TimeProjection::SourceTime {
                self.projected_messages += 1;
            }
            match record {
                IndexedRecord::Timestamp { entry, .. } => {
                    self.cache.reserve_entry(
                        &mut pending.index.entries,
                        pending.hint,
                        &mut self.building_bytes,
                        count,
                    )?;
                    pending.index.entries.push(entry);
                }
                IndexedRecord::Rejected {
                    location, error, ..
                } => {
                    pending.rejected += 1;
                    // Completion order must not change the reported last-in-file error.
                    if pending
                        .last_error
                        .as_ref()
                        .is_none_or(|(previous, _)| *previous < location)
                    {
                        pending.last_error = Some((location, error));
                    }
                }
            }
        }
        Ok(())
    }
}

fn scan_index_chunks(
    recording: &Recording,
    plans: &[(TimeProjection, Schema)],
    by_channel: &BTreeMap<u16, Vec<usize>>,
    chunks: &[u32],
    next_chunk: &AtomicUsize,
    cancelled: impl Fn() -> bool,
    build: &Mutex<IndexBuild<'_>>,
) -> Result<()> {
    let mut batch = Vec::with_capacity(INDEX_BATCH_RECORDS);
    loop {
        ensure!(!cancelled(), "indexing cancelled");
        let Some(&chunk_id) = chunks.get(next_chunk.fetch_add(1, Ordering::Relaxed)) else {
            break;
        };
        Recording::scan_chunk_at(
            &recording.file,
            &recording.summary.chunk_indexes[chunk_id as usize],
            &cancelled,
            |offset, opcode, body| {
                if opcode != mcap::records::op::MESSAGE {
                    return Ok(());
                }
                let mcap::records::Record::Message { header, data } =
                    mcap::parse_record(opcode, body)?
                else {
                    unreachable!("message opcode was checked");
                };
                let Some(slots) = by_channel.get(&header.channel_id) else {
                    return Ok(());
                };
                let source = Time::from_nanos(header.publish_time.try_into()?);
                for &index in slots {
                    let (projection, schema) = &plans[index];
                    batch.push(match projection.project_cdr(source, &data, schema) {
                        Ok(time) => IndexedRecord::Timestamp {
                            index,
                            entry: IndexEntry {
                                timestamp_ns: time.as_nanos(),
                                chunk_id,
                                record_offset: offset,
                            },
                        },
                        Err(error) => IndexedRecord::Rejected {
                            index,
                            location: (chunk_id, offset),
                            error: error.to_string(),
                        },
                    });
                    if batch.len() == INDEX_BATCH_RECORDS {
                        build
                            .lock()
                            .map_err(|_| eyre!("index build lock poisoned"))?
                            .merge(&mut batch)?;
                    }
                }
                Ok(())
            },
        )?;
        let mut build = build
            .lock()
            .map_err(|_| eyre!("index build lock poisoned"))?;
        build.merge(&mut batch)?;
        #[cfg(test)]
        {
            build.decoded_chunks += 1;
        }
    }
    Ok(())
}

/// A single coordinator owns the index cache; bounded workers scan chunks.
/// Opening uses only summary metadata; each query pins its indexes while in use.
pub struct QueryRecording {
    cancel: Arc<AtomicBool>,
    recording: Recording,
    /// Catalog instance UUID, retained across queries and cache eviction.
    publication_publisher: [u8; 16],
    cache: IndexCache,
    pub info: RecordingInfo,
    #[cfg(test)]
    pub projected_messages: usize,
    #[cfg(test)]
    index_workers: usize,
}

impl QueryRecording {
    #[cfg(test)]
    pub fn decoded_chunks(&self) -> usize {
        self.recording.decoded_chunks
    }

    #[cfg(test)]
    pub fn index_stats(&self) -> (usize, usize) {
        (self.cache.entries.len(), self.cache.bytes)
    }

    #[cfg(test)]
    pub fn with_index_workers(mut self, workers: usize) -> Self {
        assert!((1..=INDEX_WORKERS).contains(&workers));
        self.index_workers = workers;
        self
    }

    #[cfg(test)]
    pub fn has_index(&self, source: &TopicSource) -> bool {
        self.channel(&source.topic).is_ok_and(|channel| {
            self.cache
                .entries
                .contains_key(&(channel, source.projection.clone()))
        })
    }

    pub fn new(recording: Recording, name: String, cancel: Arc<AtomicBool>) -> Result<Self> {
        let instance = Uuid::new_v4();
        let info = RecordingInfo {
            instance: instance.to_string(),
            name,
            // Provisional log-time bounds require no scan. Query results provide
            // exact effective-time bounds as their projections become available.
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
            info,
            cache: IndexCache {
                entries: HashMap::new(),
                bytes: 0,
                budget: DEFAULT_INDEX_BYTES,
                clock: 0,
            },
            #[cfg(test)]
            projected_messages: 0,
            #[cfg(test)]
            index_workers: INDEX_WORKERS,
        })
    }

    pub fn with_index_budget(mut self, bytes: usize) -> Result<Self> {
        ensure!(
            bytes >= ENTRY_BYTES,
            "index cache must hold at least one {ENTRY_BYTES}-byte entry"
        );
        self.cache.budget = bytes;
        self.cache.make_room(0, 0)?;
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

    fn indexes(&mut self, keys: &[IndexKey]) -> Result<Vec<Arc<Index>>> {
        let mut available = HashMap::new();
        let mut pending = Vec::<PendingIndex>::new();
        for key in keys {
            if available.contains_key(key) || pending.iter().any(|pending| pending.key == *key) {
                continue;
            }
            if let Some(index) = self.cache.get(key) {
                available.insert(key.clone(), index);
                continue;
            }
            let channel = &self.recording.channels[&key.0];
            key.1
                .validate(&channel.schema)
                .wrap_err_with(|| format!("timestamp projection for {}", channel.topic))?;
            let hint = self
                .recording
                .summary
                .stats
                .as_ref()
                .and_then(|stats| stats.channel_message_counts.get(&key.0))
                .and_then(|count| usize::try_from(*count).ok())
                .unwrap_or(0);
            pending.push(PendingIndex {
                key: key.clone(),
                schema: Arc::clone(&channel.schema),
                index: Index::default(),
                hint,
                rejected: 0,
                last_error: None,
            });
        }
        let count = pending.len();
        self.cache.make_room(0, count)?;
        if count != 0 {
            let mut by_channel = BTreeMap::<u16, Vec<usize>>::new();
            for (slot, pending) in pending.iter().enumerate() {
                by_channel.entry(pending.key.0).or_default().push(slot);
            }
            let chunks: Vec<_> = self
                .recording
                .summary
                .chunk_indexes
                .iter()
                .enumerate()
                .filter(|(_, chunk)| {
                    chunk.message_index_offsets.is_empty()
                        || by_channel
                            .keys()
                            .any(|channel| chunk.message_index_offsets.contains_key(channel))
                })
                .map(|(index, _)| u32::try_from(index))
                .collect::<std::result::Result<_, _>>()?;
            self.build_indexes(&mut pending, &by_channel, &chunks)?;
            // Publish no partial indexes: cancellation or failure drops all builders.
            for mut pending in pending {
                ensure!(!self.cancel.load(Ordering::Relaxed), "indexing cancelled");
                pending.index.entries.sort_unstable();
                if let Some((_, error)) = pending.last_error {
                    pending.index.diagnostics.push(format!(
                        "{} records have invalid projected timestamps: {error}",
                        pending.rejected
                    ));
                }
                let index = Arc::new(pending.index);
                self.cache.insert(pending.key.clone(), Arc::clone(&index));
                available.insert(pending.key, index);
            }
        }
        Ok(keys.iter().map(|key| Arc::clone(&available[key])).collect())
    }

    fn build_indexes(
        &mut self,
        pending: &mut [PendingIndex],
        by_channel: &BTreeMap<u16, Vec<usize>>,
        chunks: &[u32],
    ) -> Result<()> {
        // Separate immutable decoding plans from mutable output. Clone each small
        // projection configuration once, not once per chunk or message.
        let plans: Vec<_> = pending
            .iter()
            .map(|index| (index.key.1.clone(), Arc::clone(&index.schema)))
            .collect();
        let build = Mutex::new(IndexBuild {
            pending,
            cache: &mut self.cache,
            building_bytes: 0,
            #[cfg(test)]
            decoded_chunks: 0,
            #[cfg(test)]
            projected_messages: 0,
        });
        let failed = AtomicBool::new(false);
        let next_chunk = AtomicUsize::new(0);
        #[cfg(not(test))]
        let worker_limit = INDEX_WORKERS;
        #[cfg(test)]
        let worker_limit = self.index_workers;
        let result = thread::scope(|scope| {
            let mut workers = Vec::new();
            let mut failure = None;
            for worker in 0..worker_limit.min(chunks.len()) {
                let recording = &self.recording;
                let cancel = &self.cancel;
                let (plans, build, failed, next_chunk) = (&plans, &build, &failed, &next_chunk);
                let spawned = thread::Builder::new()
                    .name(format!("mcap-index-{worker}"))
                    .spawn_scoped(scope, move || {
                        let result = scan_index_chunks(
                            recording,
                            plans,
                            by_channel,
                            chunks,
                            next_chunk,
                            || cancel.load(Ordering::Relaxed) || failed.load(Ordering::Relaxed),
                            build,
                        );
                        match result {
                            Err(error) if !failed.swap(true, Ordering::Relaxed) => Err(error),
                            // Preserve the originating failure rather than another worker's
                            // consequent cancellation. This flag is local to this build.
                            _ => Ok(()),
                        }
                    });
                match spawned {
                    Ok(worker) => workers.push(worker),
                    Err(error) => {
                        failed.store(true, Ordering::Relaxed);
                        failure = Some(error.into());
                        break;
                    }
                }
            }
            for worker in workers {
                let result = worker.join().unwrap_or_else(|_| {
                    failed.store(true, Ordering::Relaxed);
                    Err(eyre!("index worker panicked"))
                });
                if let Err(error) = result {
                    failure.get_or_insert(error);
                }
            }
            failure.map_or(Ok(()), Err)
        });
        #[cfg(test)]
        {
            let build = build
                .lock()
                .map_err(|_| eyre!("index build lock poisoned"))?;
            self.recording.decoded_chunks += build.decoded_chunks;
            self.projected_messages += build.projected_messages;
        }
        result
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
            // Synthetic identity: reopening even the same file creates a new publisher.
            // The cast preserves all location bits, including the sign bit.
            publication: Some((self.publication_publisher, id as i64)),
            id,
            time: entry.time(),
            source_time: Time::from_nanos(header.publish_time.try_into()?),
            log_time: Some(Time::from_nanos(header.log_time.try_into()?)),
            data: data.as_ref().into(),
        }))
    }

    /// Publish usable primary results before building optional indexes. Views pin
    /// every acquired index for this evaluation and optional failures stay local.
    pub fn query_plan(
        &mut self,
        plan: &QueryPlan,
        cursor: Time,
        mut emit: impl FnMut(Evaluation) -> bool,
    ) -> Result<()> {
        use ros_z_debug::query::plan::{SourceState, SourceView, State};
        let sources = plan.sources()?;
        let keys: Vec<_> = sources
            .iter()
            .map(|source| {
                self.channel(&source.topic)
                    .ok()
                    .map(|channel| (channel, source.projection.clone()))
            })
            .collect();
        let mut pinned: Vec<Option<Arc<Index>>> = vec![None; sources.len()];
        let mut failures: Vec<Option<String>> = vec![None; sources.len()];
        loop {
            ensure!(!self.cancel.load(Ordering::Relaxed), "query cancelled");
            let views: Vec<_> = keys
                .iter()
                .enumerate()
                .map(|(i, key)| {
                    let Some(key) = key else {
                        return SourceState::Absent;
                    };
                    if let Some(error) = &failures[i] {
                        return SourceState::Failed(error.clone());
                    }
                    if pinned[i].is_none() {
                        pinned[i] = self.cache.get(key);
                    }
                    let Some(index) = &pinned[i] else {
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
                })
                .collect();
            let result = plan.evaluate_batched(cursor, &views, |locations| {
                let mut ordered: Vec<_> = locations
                    .iter()
                    .enumerate()
                    .map(|(output, &(source, slot))| {
                        (
                            pinned[source].as_ref().expect("pinned source").entries[slot].chunk_id,
                            source,
                            slot,
                            output,
                        )
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
            if !emit(result) {
                return Ok(());
            }
            let Some(index) = pending else {
                return Ok(());
            };
            match self.indexes(from_ref(keys[index].as_ref().expect("pending source key"))) {
                Ok(mut indexes) => pinned[index] = indexes.pop(),
                Err(error) => failures[index] = Some(format!("{error:#}")),
            }
        }
    }
}
