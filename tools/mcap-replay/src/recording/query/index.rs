use std::{
    collections::{HashMap, hash_map::Entry},
    io::Error,
    mem::size_of,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{Sender, channel},
    },
    thread,
};

use color_eyre::eyre::{Context, Result, bail, ensure};
use mcap::{
    parse_record,
    records::{Record, op::MESSAGE},
};
use parking_lot::Mutex;
use ros_z::{dynamic::Schema, time::Time};
use ros_z_debug::query::{TimeProjection, plan::TimeIndex};
use tokio::{
    runtime::Handle,
    sync::{Mutex as AsyncMutex, OwnedMutexGuard},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::super::Recording;

#[cfg(test)]
mod tests;

const MAX_INDEXES: usize = 256;
const ENTRY_BYTES: usize = size_of::<IndexEntry>();
pub(super) const INDEX_WORKERS: usize = 4;
const INDEX_BATCH_RECORDS: usize = 1024;

/// File location breaks equal-time ties independently of worker completion order.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct IndexEntry {
    timestamp_ns: i64,
    pub chunk_id: u32,
    pub record_offset: u32,
}

const _: () = assert!(ENTRY_BYTES == 16);

impl IndexEntry {
    pub fn time(&self) -> Time {
        Time::from_nanos(self.timestamp_ns)
    }
}

#[derive(Default)]
pub(super) struct Index {
    pub entries: Vec<IndexEntry>,
    pub diagnostics: Vec<String>,
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

pub(super) type IndexKey = (u16, TimeProjection);

pub(super) struct CachedIndex {
    index: Arc<Index>,
    last_used: u64,
}

#[derive(Default)]
pub(super) struct BuildStats {
    pub decoded_chunks: usize,
    pub projected_messages: usize,
}

pub struct IndexCache {
    pub(super) state: Arc<Mutex<CacheState>>,
    pub(super) builds: Arc<AsyncMutex<()>>,
    cleanup: Sender<Uuid>,
}

pub(super) struct CacheState {
    pub(super) entries: HashMap<(Uuid, IndexKey), CachedIndex>,
    // Includes removed indexes until the cleanup worker has released their allocations.
    pub bytes: usize,
    pub(super) stats: BuildStats,
    budget: usize,
    clock: u64,
}

impl CacheState {
    fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            stats: BuildStats::default(),
            budget,
            clock: 0,
        }
    }

    pub(super) fn get(&mut self, owner: &Uuid, key: &IndexKey) -> Option<Arc<Index>> {
        let cached = self.entries.get_mut(&(*owner, key.clone()))?;
        self.clock = self.clock.saturating_add(1);
        cached.last_used = self.clock;
        Some(Arc::clone(&cached.index))
    }

    /// Held Arcs pin query inputs; unfinished builders are charged separately.
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
        let growth = match entries.capacity() {
            0 if hint > 0 && hint <= available => hint,
            0 => 256.min(available),
            capacity => capacity.min(available),
        };
        self.make_room(
            building_bytes.saturating_add(growth * ENTRY_BYTES),
            building_count,
        )?;
        let previous = entries.capacity();
        entries.try_reserve_exact(growth)?;
        *building_bytes += (entries.capacity() - previous) * ENTRY_BYTES;
        // Count actual allocation, including any allocator over-reservation.
        self.make_room(*building_bytes, building_count)
    }

    fn remove_recording(&mut self, owner: Uuid) -> Vec<Arc<Index>> {
        self.entries
            .extract_if(|(id, _), _| *id == owner)
            .map(|(_, cached)| cached.index)
            .collect()
    }

    fn insert(&mut self, owner: &Uuid, key: IndexKey, index: Arc<Index>) -> Result<()> {
        let bytes = self
            .bytes
            .checked_add(index.bytes())
            .filter(|bytes| *bytes <= self.budget)
            .ok_or_else(|| Error::other("index cache budget exceeded"))?;
        let Entry::Vacant(entry) = self.entries.entry((*owner, key)) else {
            bail!("index already cached");
        };
        self.clock = self.clock.saturating_add(1);
        entry.insert(CachedIndex {
            index,
            last_used: self.clock,
        });
        self.bytes = bytes;
        Ok(())
    }
}

impl IndexCache {
    pub fn new(budget: usize) -> Result<(Self, JoinHandle<Result<()>>)> {
        ensure!(
            budget >= ENTRY_BYTES,
            "index cache must hold at least one index entry"
        );
        let state = Arc::new(Mutex::new(CacheState::new(budget)));
        let (cleanup, removed) = channel();
        let worker_state = Arc::clone(&state);
        let worker = Handle::try_current()
            .wrap_err("starting index cleanup task")?
            .spawn_blocking(move || {
                for owner in removed {
                    let indexes = worker_state.lock().remove_recording(owner);
                    let bytes = indexes.iter().map(|index| index.bytes()).sum::<usize>();
                    // Release potentially large allocations outside the cache lock.
                    drop(indexes);
                    worker_state.lock().bytes -= bytes;
                }
                Ok(())
            });

        Ok((
            Self {
                state,
                builds: Arc::new(AsyncMutex::new(())),
                cleanup,
            },
            worker,
        ))
    }

    pub(super) fn remove_recording(&self, owner: Uuid) {
        if let Err(error) = self.cleanup.send(owner) {
            tracing::error!(%owner, %error, "index cleanup worker stopped unexpectedly");
        }
    }

    pub(super) fn get(&self, owner: &Uuid, key: &IndexKey) -> Option<Arc<Index>> {
        self.state.lock().get(owner, key)
    }

    pub(super) async fn acquire_build(&self) -> OwnedMutexGuard<()> {
        Arc::clone(&self.builds).lock_owned().await
    }

    pub(super) fn index(
        &self,
        owner: &Uuid,
        recording: &Recording,
        cancel: &CancellationToken,
        workers: usize,
        key: &IndexKey,
        _guard: OwnedMutexGuard<()>,
    ) -> Result<Arc<Index>> {
        // The owned guard outlives unfinished allocations, including on failure.
        ensure!(!cancel.is_cancelled(), "indexing cancelled");
        if let Some(index) = self.get(owner, key) {
            return Ok(index);
        }
        let mut pending = PendingIndex::new(recording, key.clone())?;
        self.state.lock().make_room(0, 1)?;
        self.build_index(recording, cancel, workers, &mut pending)?;
        ensure!(!cancel.is_cancelled(), "indexing cancelled");
        let index = pending.finish();
        self.state
            .lock()
            .insert(owner, key.clone(), Arc::clone(&index))?;
        Ok(index)
    }

    fn build_index(
        &self,
        recording: &Recording,
        cancel: &CancellationToken,
        workers: usize,
        pending: &mut PendingIndex,
    ) -> Result<()> {
        let scan = IndexScan::new(recording, cancel, pending)?;
        let build = Mutex::new(IndexBuild {
            pending,
            cache: self,
            building_bytes: 0,
            stats: BuildStats::default(),
        });
        let result = scan.run(&build, workers);
        let build = build.into_inner();
        let mut state = self.state.lock();
        state.stats.decoded_chunks += build.stats.decoded_chunks;
        state.stats.projected_messages += build.stats.projected_messages;
        result
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

impl PendingIndex {
    fn new(recording: &Recording, key: IndexKey) -> Result<Self> {
        let channel = &recording.channels[&key.0];
        key.1
            .validate(&channel.schema)
            .wrap_err_with(|| format!("timestamp projection for {}", channel.topic))?;
        let hint = recording
            .summary
            .stats
            .as_ref()
            .and_then(|stats| stats.channel_message_counts.get(&key.0))
            .and_then(|count| usize::try_from(*count).ok())
            .unwrap_or(0);
        Ok(Self {
            key,
            schema: Arc::clone(&channel.schema),
            index: Index::default(),
            hint,
            rejected: 0,
            last_error: None,
        })
    }

    fn reject(&mut self, location: (u32, u32), error: String) {
        self.rejected += 1;
        // Report the last error in file order, not worker completion order.
        if self
            .last_error
            .as_ref()
            .is_none_or(|(previous, _)| *previous < location)
        {
            self.last_error = Some((location, error));
        }
    }

    fn finish(mut self) -> Arc<Index> {
        self.index.entries.sort_unstable();
        if let Some((_, error)) = self.last_error {
            self.index.diagnostics.push(format!(
                "{} records have invalid projected timestamps: {error}",
                self.rejected
            ));
        }
        Arc::new(self.index)
    }
}

enum IndexedRecord {
    Timestamp(IndexEntry),
    Rejected { location: (u32, u32), error: String },
}

/// All allocation and eviction happens under this lock, never file reads or projection.
struct IndexBuild<'a> {
    pending: &'a mut PendingIndex,
    cache: &'a IndexCache,
    building_bytes: usize,
    stats: BuildStats,
}

impl IndexBuild<'_> {
    fn merge(&mut self, batch: &mut Vec<IndexedRecord>) -> Result<()> {
        let pending = &mut self.pending;
        for record in batch.drain(..) {
            if pending.key.1 != TimeProjection::SourceTime {
                self.stats.projected_messages += 1;
            }
            match record {
                IndexedRecord::Timestamp(entry) => {
                    if pending.index.entries.len() == pending.index.entries.capacity() {
                        self.cache.state.lock().reserve_entry(
                            &mut pending.index.entries,
                            pending.hint,
                            &mut self.building_bytes,
                            1,
                        )?;
                    }
                    pending.index.entries.push(entry);
                }
                IndexedRecord::Rejected {
                    location, error, ..
                } => pending.reject(location, error),
            }
        }
        Ok(())
    }
}

struct IndexScan<'a> {
    recording: &'a Recording,
    cancel: &'a CancellationToken,
    key: IndexKey,
    schema: Schema,
    chunks: Vec<u32>,
    next_chunk: AtomicUsize,
    failed: AtomicBool,
}

impl<'a> IndexScan<'a> {
    fn new(
        recording: &'a Recording,
        cancel: &'a CancellationToken,
        pending: &PendingIndex,
    ) -> Result<Self> {
        let chunks = recording
            .summary
            .chunk_indexes
            .iter()
            .enumerate()
            .filter(|(_, chunk)| {
                chunk.message_index_offsets.is_empty()
                    || chunk.message_index_offsets.contains_key(&pending.key.0)
            })
            .map(|(index, _)| u32::try_from(index))
            .collect::<std::result::Result<_, _>>()?;
        Ok(Self {
            recording,
            cancel,
            key: pending.key.clone(),
            schema: Arc::clone(&pending.schema),
            chunks,
            next_chunk: AtomicUsize::new(0),
            failed: AtomicBool::new(false),
        })
    }

    fn run(&self, build: &Mutex<IndexBuild<'_>>, worker_limit: usize) -> Result<()> {
        thread::scope(|scope| {
            let mut workers = Vec::new();
            let mut failure = None;
            for worker in 0..worker_limit.min(self.chunks.len()) {
                let spawned = thread::Builder::new()
                    .name(format!("mcap-index-{worker}"))
                    .spawn_scoped(scope, || self.run_worker(build));
                match spawned {
                    Ok(worker) => workers.push(worker),
                    Err(error) => {
                        self.failed.store(true, Ordering::Relaxed);
                        failure = Some(error.into());
                        break;
                    }
                }
            }
            for worker in workers {
                let result = worker.join().unwrap_or_else(|_| {
                    self.failed.store(true, Ordering::Relaxed);
                    Err(Error::other("index worker panicked")).wrap_err("joining index worker")
                });
                if let Err(error) = result {
                    failure.get_or_insert(error);
                }
            }
            failure.map_or(Ok(()), Err)
        })
    }

    fn run_worker(&self, build: &Mutex<IndexBuild<'_>>) -> Result<()> {
        match self.scan_chunks(build) {
            Err(error) if !self.failed.swap(true, Ordering::Relaxed) => Err(error),
            // Preserve the originating failure instead of another worker's cancellation.
            _ => Ok(()),
        }
    }

    fn cancelled(&self) -> bool {
        self.cancel.is_cancelled() || self.failed.load(Ordering::Relaxed)
    }

    fn scan_chunks(&self, build: &Mutex<IndexBuild<'_>>) -> Result<()> {
        let mut batch = Vec::with_capacity(INDEX_BATCH_RECORDS);
        loop {
            ensure!(!self.cancelled(), "indexing cancelled");
            let Some(&chunk_id) = self
                .chunks
                .get(self.next_chunk.fetch_add(1, Ordering::Relaxed))
            else {
                return Ok(());
            };
            Recording::scan_chunk_at(
                &self.recording.file,
                &self.recording.summary.chunk_indexes[chunk_id as usize],
                || self.cancelled(),
                |offset, opcode, body| {
                    self.project_message(chunk_id, offset, opcode, body, &mut batch, build)
                },
            )?;
            let mut build = build.lock();
            build.merge(&mut batch)?;
            build.stats.decoded_chunks += 1;
        }
    }

    fn project_message(
        &self,
        chunk_id: u32,
        offset: u32,
        opcode: u8,
        body: &[u8],
        batch: &mut Vec<IndexedRecord>,
        build: &Mutex<IndexBuild<'_>>,
    ) -> Result<()> {
        if opcode != MESSAGE {
            return Ok(());
        }
        let Record::Message { header, data } = parse_record(opcode, body)? else {
            unreachable!("message opcode was checked");
        };
        if header.channel_id != self.key.0 {
            return Ok(());
        }
        let source = Time::from_nanos(header.publish_time.try_into()?);
        batch.push(match self.key.1.project_cdr(source, &data, &self.schema) {
            Ok(time) => IndexedRecord::Timestamp(IndexEntry {
                timestamp_ns: time.as_nanos(),
                chunk_id,
                record_offset: offset,
            }),
            Err(error) => IndexedRecord::Rejected {
                location: (chunk_id, offset),
                error: error.to_string(),
            },
        });
        if batch.len() == INDEX_BATCH_RECORDS {
            build.lock().merge(batch)?;
        }
        Ok(())
    }
}
