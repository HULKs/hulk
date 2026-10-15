use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};

use color_eyre::eyre::{Context, Result, bail, ensure, eyre};
use mcap::{
    parse_record,
    records::{Record, op::MESSAGE},
};
use ros_z::{dynamic::Schema, time::Time};
use ros_z_debug::query::{TimeProjection, plan::TimeIndex};

use super::super::Recording;

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

pub(super) struct IndexCache {
    pub entries: HashMap<IndexKey, CachedIndex>,
    pub bytes: usize,
    pub stats: BuildStats,
    budget: usize,
    clock: u64,
}

impl IndexCache {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            stats: BuildStats::default(),
            budget,
            clock: 0,
        }
    }

    pub fn set_budget(&mut self, bytes: usize) -> Result<()> {
        ensure!(
            bytes >= ENTRY_BYTES,
            "index cache must hold at least one {ENTRY_BYTES}-byte entry"
        );
        self.budget = bytes;
        self.make_room(0, 0)
    }

    pub fn get(&mut self, key: &IndexKey) -> Option<Arc<Index>> {
        let cached = self.entries.get_mut(key)?;
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

    pub fn indexes(
        &mut self,
        recording: &Recording,
        cancel: &AtomicBool,
        workers: usize,
        keys: &[IndexKey],
    ) -> Result<Vec<Arc<Index>>> {
        let mut available = HashMap::new();
        let mut pending = Vec::<PendingIndex>::new();
        for key in keys {
            if available.contains_key(key) || pending.iter().any(|pending| pending.key == *key) {
                continue;
            }
            if let Some(index) = self.get(key) {
                available.insert(key.clone(), index);
                continue;
            }
            pending.push(PendingIndex::new(recording, key.clone())?);
        }
        self.make_room(0, pending.len())?;
        if !pending.is_empty() {
            self.build_indexes(recording, cancel, workers, &mut pending)?;
        }
        // A failed build drops every unfinished index instead of caching partial data.
        for pending in pending {
            ensure!(!cancel.load(Ordering::Relaxed), "indexing cancelled");
            let (key, index) = pending.finish();
            self.insert(key.clone(), Arc::clone(&index));
            available.insert(key, index);
        }
        Ok(keys.iter().map(|key| Arc::clone(&available[key])).collect())
    }

    fn build_indexes(
        &mut self,
        recording: &Recording,
        cancel: &AtomicBool,
        workers: usize,
        pending: &mut [PendingIndex],
    ) -> Result<()> {
        let scan = IndexScan::new(recording, cancel, pending)?;
        let build = Mutex::new(IndexBuild {
            pending,
            cache: self,
            building_bytes: 0,
            stats: BuildStats::default(),
        });
        let result = scan.run(&build, workers);
        let build = build
            .into_inner()
            .map_err(|_| eyre!("index build lock poisoned"))?;
        build.cache.stats.decoded_chunks += build.stats.decoded_chunks;
        build.cache.stats.projected_messages += build.stats.projected_messages;
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

    fn finish(mut self) -> (IndexKey, Arc<Index>) {
        self.index.entries.sort_unstable();
        if let Some((_, error)) = self.last_error {
            self.index.diagnostics.push(format!(
                "{} records have invalid projected timestamps: {error}",
                self.rejected
            ));
        }
        (self.key, Arc::new(self.index))
    }
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

/// All allocation and eviction happens under this lock, never file reads or projection.
struct IndexBuild<'a> {
    pending: &'a mut [PendingIndex],
    cache: &'a mut IndexCache,
    building_bytes: usize,
    stats: BuildStats,
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
            if pending.key.1 != TimeProjection::SourceTime {
                self.stats.projected_messages += 1;
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
                } => pending.reject(location, error),
            }
        }
        Ok(())
    }
}

struct IndexScan<'a> {
    recording: &'a Recording,
    cancel: &'a AtomicBool,
    plans: Vec<(TimeProjection, Schema)>,
    by_channel: BTreeMap<u16, Vec<usize>>,
    chunks: Vec<u32>,
    next_chunk: AtomicUsize,
    failed: AtomicBool,
}

impl<'a> IndexScan<'a> {
    fn new(
        recording: &'a Recording,
        cancel: &'a AtomicBool,
        pending: &[PendingIndex],
    ) -> Result<Self> {
        let mut by_channel = BTreeMap::<u16, Vec<usize>>::new();
        for (slot, pending) in pending.iter().enumerate() {
            by_channel.entry(pending.key.0).or_default().push(slot);
        }
        let chunks = recording
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
        // Copy each small projection once, not once per chunk or message.
        let plans = pending
            .iter()
            .map(|index| (index.key.1.clone(), Arc::clone(&index.schema)))
            .collect();
        Ok(Self {
            recording,
            cancel,
            plans,
            by_channel,
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
                    Err(eyre!("index worker panicked"))
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
        self.cancel.load(Ordering::Relaxed) || self.failed.load(Ordering::Relaxed)
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
            let mut build = build
                .lock()
                .map_err(|_| eyre!("index build lock poisoned"))?;
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
        let Some(slots) = self.by_channel.get(&header.channel_id) else {
            return Ok(());
        };
        let source = Time::from_nanos(header.publish_time.try_into()?);
        for &index in slots {
            let (projection, schema) = &self.plans[index];
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
                    .merge(batch)?;
            }
        }
        Ok(())
    }
}
