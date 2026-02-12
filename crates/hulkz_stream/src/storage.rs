use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::BufWriter,
    path::{Path, PathBuf},
    sync::Arc,
};

use enumset::enum_set;
use hulkz::Timestamp;
use mcap::{
    read::{MessageStream, Options, Summary},
    records::{MessageHeader, MessageIndexEntry},
    write::{WriteOptions, Writer},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, trace, warn};

use crate::{
    error::{Error, Result},
    keyspace::{
        encoding_from_mcap, encoding_to_mcap, from_nanos_with_id, key_expr_for_record,
        metadata_for_record, source_from_topic_and_metadata, source_key,
        timestamp_id_from_metadata, to_nanos,
    },
    types::{OpenMode, SourceSpec, SourceStats, StreamRecord},
};

/// Current on-disk manifest schema version.
pub const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct DurableStats {
    /// Oldest durable timestamp indexed for a source.
    pub oldest: Option<Timestamp>,
    /// Latest durable timestamp indexed for a source.
    pub latest: Option<Timestamp>,
    /// Total durable message count indexed for a source.
    pub len: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Manifest {
    version: u32,
    next_segment_id: u64,
    active_segment_id: Option<u64>,
    segments: Vec<ManifestSegment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ManifestSegment {
    id: u64,
    file: String,
    sealed: bool,
}

struct ActiveWriter {
    segment_id: u64,
    path: PathBuf,
    writer: Writer<BufWriter<File>>,
    channels: HashMap<String, u16>,
    next_sequence: u32,
}

#[derive(Debug, Clone, Copy)]
struct DurablePointer {
    timestamp_nanos: u64,
    segment_id: u64,
    message_index: usize,
}

#[derive(Debug, Clone)]
enum MessageLocator {
    Summary {
        chunk_index: usize,
        entry: MessageIndexEntry,
    },
    InMemoryPayload(Arc<[u8]>),
}

#[derive(Debug, Clone)]
struct SegmentIndexedMessage {
    source: SourceSpec,
    effective_namespace: Option<String>,
    timestamp_nanos: u64,
    timestamp_id: Option<String>,
    encoding: zenoh::bytes::Encoding,
    locator: MessageLocator,
}

#[derive(Debug)]
struct SegmentIndex {
    segment_id: u64,
    path: PathBuf,
    by_source: HashMap<String, Vec<usize>>,
    messages: Vec<SegmentIndexedMessage>,
}

struct StorageState {
    mode: OpenMode,
    segments_path: PathBuf,
    manifest_path: Option<PathBuf>,
    manifest: Option<Manifest>,
    max_segment_bytes: u64,
    active_writer: Option<ActiveWriter>,
    active_records: Vec<StreamRecord>,
    active_by_source: HashMap<String, Vec<usize>>,
    durable_stats: HashMap<String, DurableStats>,
    external_segments: Vec<PathBuf>,
    sealed_indexes: HashMap<u64, SegmentIndex>,
    source_index: HashMap<String, Vec<DurablePointer>>,
    next_external_segment_id: u64,
}

#[derive(Clone)]
pub struct Storage {
    inner: Arc<Mutex<StorageState>>,
}

impl Storage {
    /// Opens managed storage (manifest + segments) or an external read-only MCAP path.
    pub async fn open(mode: OpenMode, path: PathBuf, max_segment_bytes: u64) -> Result<Self> {
        info!(
            ?mode,
            storage_path = %path.display(),
            max_segment_bytes,
            "opening storage",
        );
        let mut state = if path.is_file() {
            if mode != OpenMode::ReadOnly {
                return Err(Error::InvalidStoragePath(path));
            }
            info!("opening read-only external MCAP file");
            let mut state = StorageState {
                mode,
                segments_path: path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                manifest_path: None,
                manifest: None,
                max_segment_bytes,
                active_writer: None,
                active_records: Vec::new(),
                active_by_source: HashMap::new(),
                durable_stats: HashMap::new(),
                external_segments: vec![path],
                sealed_indexes: HashMap::new(),
                source_index: HashMap::new(),
                next_external_segment_id: 1,
            };
            state.rebuild_indexes_and_stats()?;
            state
        } else {
            if !path.exists() {
                if mode == OpenMode::ReadWrite {
                    fs::create_dir_all(&path)?;
                } else {
                    return Err(Error::InvalidStoragePath(path));
                }
            }

            let manifest_path = path.join("manifest.json");
            let segments_path = path.join("segments");
            if mode == OpenMode::ReadWrite {
                fs::create_dir_all(&segments_path)?;
            }

            let mut state = StorageState {
                mode,
                segments_path,
                manifest_path: Some(manifest_path.clone()),
                manifest: None,
                max_segment_bytes,
                active_writer: None,
                active_records: Vec::new(),
                active_by_source: HashMap::new(),
                durable_stats: HashMap::new(),
                external_segments: Vec::new(),
                sealed_indexes: HashMap::new(),
                source_index: HashMap::new(),
                next_external_segment_id: 1,
            };

            if manifest_path.exists() {
                state.manifest = Some(state.load_manifest()?);
            }

            if mode == OpenMode::ReadOnly {
                if state.manifest.is_none() {
                    state.external_segments = discover_external_segments(&path)?;
                    info!(
                        count = state.external_segments.len(),
                        "discovered external segments for read-only storage",
                    );
                }
                state.rebuild_indexes_and_stats()?;
            } else {
                if state.manifest.is_none() {
                    info!("initializing new storage manifest");
                    state.manifest = Some(Manifest {
                        version: MANIFEST_VERSION,
                        next_segment_id: 1,
                        active_segment_id: None,
                        segments: Vec::new(),
                    });
                }

                state.recover_unsealed_segments()?;
                state.rebuild_indexes_and_stats()?;
                state.start_new_active_segment()?;
                state.save_manifest()?;
            }

            state
        };

        if mode == OpenMode::ReadWrite {
            state.active_records.clear();
            state.active_by_source.clear();
        }

        info!(
            durable_sources = state.durable_stats.len(),
            sealed_segments = state.sealed_indexes.len(),
            "storage open complete",
        );

        Ok(Self {
            inner: Arc::new(Mutex::new(state)),
        })
    }

    /// Appends a new durable record in read-write mode.
    pub async fn append(&self, record: StreamRecord) -> Result<DurableStats> {
        let mut guard = self.inner.lock().await;
        if guard.mode == OpenMode::ReadOnly {
            return Err(Error::ReadOnly);
        }

        guard.append_record(record)
    }

    pub async fn durable_stats(&self, spec: &SourceSpec) -> Option<DurableStats> {
        let guard = self.inner.lock().await;
        guard.durable_stats.get(&source_key(spec)).cloned()
    }

    /// Returns newest durable record for the source.
    pub async fn query_latest(&self, spec: &SourceSpec) -> Result<Option<StreamRecord>> {
        let guard = self.inner.lock().await;
        let source = source_key(spec);

        let sealed_candidate = guard
            .source_index
            .get(&source)
            .and_then(|pointers| pointers.last().copied());
        let active_candidate = guard
            .active_by_source
            .get(&source)
            .and_then(|indices| indices.last().copied())
            .and_then(|index| guard.active_records.get(index).cloned());

        let sealed_record = match sealed_candidate {
            Some(pointer) => Some(guard.fetch_pointer(pointer)?),
            None => None,
        };

        Ok(match (sealed_record, active_candidate) {
            (Some(sealed), Some(active)) => {
                if active.timestamp >= sealed.timestamp {
                    Some(active)
                } else {
                    Some(sealed)
                }
            }
            (Some(sealed), None) => Some(sealed),
            (None, Some(active)) => Some(active),
            (None, None) => None,
        })
    }

    /// Returns newest durable record with timestamp <= target.
    pub async fn query_before_or_equal(
        &self,
        spec: &SourceSpec,
        timestamp: Timestamp,
    ) -> Result<Option<StreamRecord>> {
        let guard = self.inner.lock().await;
        let source = source_key(spec);
        let target = to_nanos(&timestamp);

        let sealed_candidate = guard
            .source_index
            .get(&source)
            .and_then(|pointers| find_before_or_equal_pointer(pointers, target));
        let active_candidate = guard.active_by_source.get(&source).and_then(|indices| {
            indices
                .iter()
                .copied()
                .filter_map(|index| guard.active_records.get(index).cloned())
                .filter(|record| record.timestamp <= timestamp)
                .max_by_key(|record| record.timestamp)
        });

        let sealed_record = match sealed_candidate {
            Some(pointer) => Some(guard.fetch_pointer(pointer)?),
            None => None,
        };

        Ok(match (sealed_record, active_candidate) {
            (Some(sealed), Some(active)) => {
                if active.timestamp >= sealed.timestamp {
                    Some(active)
                } else {
                    Some(sealed)
                }
            }
            (Some(sealed), None) => Some(sealed),
            (None, Some(active)) => Some(active),
            (None, None) => None,
        })
    }

    /// Returns durable record nearest to target timestamp (earlier on ties).
    pub async fn query_nearest(
        &self,
        spec: &SourceSpec,
        timestamp: Timestamp,
    ) -> Result<Option<StreamRecord>> {
        let guard = self.inner.lock().await;
        let source = source_key(spec);
        let target = to_nanos(&timestamp);

        let sealed_candidates = guard
            .source_index
            .get(&source)
            .map(|pointers| find_neighbor_pointers(pointers, target))
            .unwrap_or_default();

        let mut candidates = Vec::new();
        for pointer in sealed_candidates.into_iter().flatten() {
            candidates.push(guard.fetch_pointer(pointer)?);
        }

        if let Some(indices) = guard.active_by_source.get(&source) {
            for index in indices {
                if let Some(record) = guard.active_records.get(*index) {
                    candidates.push(record.clone());
                }
            }
        }

        Ok(candidates.into_iter().min_by(|a, b| {
            let diff_a = timestamp
                .get_time()
                .to_duration()
                .abs_diff(a.timestamp.get_time().to_duration());
            let diff_b = timestamp
                .get_time()
                .to_duration()
                .abs_diff(b.timestamp.get_time().to_duration());
            diff_a
                .cmp(&diff_b)
                .then_with(|| a.timestamp.cmp(&b.timestamp))
        }))
    }

    /// Returns durable records in inclusive range, sorted by timestamp.
    pub async fn query_range_inclusive(
        &self,
        spec: &SourceSpec,
        start: Timestamp,
        end: Timestamp,
    ) -> Result<Vec<StreamRecord>> {
        let guard = self.inner.lock().await;
        let source = source_key(spec);
        let start_nanos = to_nanos(&start);
        let end_nanos = to_nanos(&end);

        let pointers = guard
            .source_index
            .get(&source)
            .map(|entries| select_range_pointers(entries, start_nanos, end_nanos))
            .unwrap_or_default();

        let mut records = guard.fetch_pointers(&pointers)?;

        if let Some(indices) = guard.active_by_source.get(&source) {
            records.extend(
                indices
                    .iter()
                    .copied()
                    .filter_map(|index| guard.active_records.get(index).cloned())
                    .filter(|record| record.timestamp >= start && record.timestamp <= end),
            );
        }

        records.sort_by_key(|record| record.timestamp);
        Ok(records)
    }

    /// Returns durable records across all sources in inclusive range.
    pub async fn query_range_all(
        &self,
        start: Timestamp,
        end: Timestamp,
    ) -> Result<Vec<StreamRecord>> {
        let guard = self.inner.lock().await;
        let start_nanos = to_nanos(&start);
        let end_nanos = to_nanos(&end);

        let mut pointers = Vec::new();
        for source_entries in guard.source_index.values() {
            pointers.extend(select_range_pointers(
                source_entries,
                start_nanos,
                end_nanos,
            ));
        }

        let mut records = guard.fetch_pointers(&pointers)?;
        records.extend(
            guard
                .active_records
                .iter()
                .filter(|record| record.timestamp >= start && record.timestamp <= end)
                .cloned(),
        );

        records.sort_by_key(|record| record.timestamp);
        records.dedup_by_key(|record| {
            (
                source_key(&record.source),
                record.effective_namespace.clone(),
                to_nanos(&record.timestamp),
                record.encoding.to_string(),
                record.payload.clone(),
            )
        });

        Ok(records)
    }

    pub async fn shutdown(&self) -> Result<()> {
        let mut guard = self.inner.lock().await;
        guard.shutdown()
    }

    pub async fn source_stats_snapshot(&self, spec: &SourceSpec) -> SourceStats {
        let guard = self.inner.lock().await;
        let durable = guard.durable_stats.get(&source_key(spec));
        SourceStats {
            durable_oldest: durable.and_then(|d| d.oldest),
            durable_latest: durable.and_then(|d| d.latest),
            durable_len: durable.map(|d| d.len).unwrap_or(0),
            ingest_frontier: None,
            durable_frontier: durable.and_then(|d| d.latest),
            last_error: None,
        }
    }

    pub async fn durable_global_frontier(&self) -> Option<Timestamp> {
        let guard = self.inner.lock().await;
        guard.durable_stats.values().filter_map(|s| s.latest).max()
    }
}

impl StorageState {
    fn append_record(&mut self, record: StreamRecord) -> Result<DurableStats> {
        let active = self.active_writer.as_mut().ok_or(Error::BackendClosed)?;

        let topic = key_expr_for_record(&record.source, record.effective_namespace.as_deref());
        let metadata = metadata_for_record(&record);
        let channel_key = format!(
            "{}|{}|{}",
            topic,
            encoding_to_mcap(&record.encoding),
            serde_json::to_string(&metadata)?,
        );

        let channel_id = if let Some(channel_id) = active.channels.get(&channel_key).copied() {
            channel_id
        } else {
            let channel_id = active.writer.add_channel(
                0,
                &topic,
                &encoding_to_mcap(&record.encoding),
                &metadata,
            )?;
            active.channels.insert(channel_key, channel_id);
            channel_id
        };

        let nanos = to_nanos(&record.timestamp);
        let header = MessageHeader {
            channel_id,
            sequence: active.next_sequence,
            log_time: nanos,
            publish_time: nanos,
        };
        active.next_sequence = active.next_sequence.wrapping_add(1);

        active
            .writer
            .write_to_known_channel(&header, &record.payload)?;
        active.writer.flush()?;
        OpenOptions::new()
            .write(true)
            .open(&active.path)?
            .sync_data()?;

        let source = source_key(&record.source);
        let updated_stats = {
            let stats = self
                .durable_stats
                .entry(source.clone())
                .or_insert(DurableStats {
                    oldest: None,
                    latest: None,
                    len: 0,
                });
            stats.oldest = Some(match stats.oldest {
                Some(oldest) => oldest.min(record.timestamp),
                None => record.timestamp,
            });
            stats.latest = Some(match stats.latest {
                Some(latest) => latest.max(record.timestamp),
                None => record.timestamp,
            });
            stats.len = stats.len.saturating_add(1);
            stats.clone()
        };

        let index = self.active_records.len();
        self.active_records.push(record);
        self.active_by_source
            .entry(source.clone())
            .or_default()
            .push(index);

        let bytes = fs::metadata(&active.path)?.len();
        if bytes >= self.max_segment_bytes {
            info!(
                segment_id = active.segment_id,
                segment_bytes = bytes,
                max_segment_bytes = self.max_segment_bytes,
                "rolling active segment",
            );
            self.roll_active_segment()?;
        }

        self.save_manifest()?;
        trace!(
            source_key = %source,
            durable_len = updated_stats.len,
            "appended durable record",
        );

        Ok(updated_stats)
    }

    fn recover_unsealed_segments(&mut self) -> Result<()> {
        let Some(manifest) = self.manifest.as_mut() else {
            return Ok(());
        };
        let mut recovered = 0usize;

        for segment in &mut manifest.segments {
            if segment.sealed {
                continue;
            }

            let path = self.segments_path.join(&segment.file);
            // Validate as incomplete and then seal to keep restart behavior deterministic.
            let _ = build_segment_index(segment.id, path, true)?;
            segment.sealed = true;
            recovered = recovered.saturating_add(1);
        }

        manifest.active_segment_id = None;
        if recovered > 0 {
            info!(recovered, "recovered unsealed segments");
        }
        Ok(())
    }

    fn start_new_active_segment(&mut self) -> Result<()> {
        let manifest = self.manifest.as_mut().ok_or(Error::BackendClosed)?;
        let segment_id = manifest.next_segment_id;
        manifest.next_segment_id = manifest.next_segment_id.saturating_add(1);

        let file = format!("segment-{segment_id:06}.mcap");
        let path = self.segments_path.join(&file);
        let writer = WriteOptions::default().create(BufWriter::new(File::create(&path)?))?;

        manifest.segments.push(ManifestSegment {
            id: segment_id,
            file,
            sealed: false,
        });
        manifest.active_segment_id = Some(segment_id);

        self.active_writer = Some(ActiveWriter {
            segment_id,
            path,
            writer,
            channels: HashMap::new(),
            next_sequence: 0,
        });
        debug!(segment_id, "started new active segment");

        self.active_records.clear();
        self.active_by_source.clear();
        Ok(())
    }

    fn roll_active_segment(&mut self) -> Result<()> {
        if let Some(mut active) = self.active_writer.take() {
            let sealed_segment_id = active.segment_id;
            let sealed_path = active.path.clone();
            active.writer.finish()?;
            debug!(segment_id = sealed_segment_id, "sealed active segment");

            if let Some(manifest) = self.manifest.as_mut() {
                if let Some(segment) = manifest
                    .segments
                    .iter_mut()
                    .find(|segment| segment.id == sealed_segment_id)
                {
                    segment.sealed = true;
                }
                manifest.active_segment_id = None;
            }

            let sealed_index = build_segment_index(sealed_segment_id, sealed_path, false)?;
            self.add_segment_index(sealed_index);
        }

        self.active_records.clear();
        self.active_by_source.clear();
        self.start_new_active_segment()?;
        Ok(())
    }

    fn rebuild_indexes_and_stats(&mut self) -> Result<()> {
        debug!("rebuilding storage indexes and durable stats");
        self.sealed_indexes.clear();
        self.source_index.clear();
        self.durable_stats.clear();

        if let Some(manifest) = &self.manifest {
            let segments: Vec<ManifestSegment> = manifest.segments.to_vec();
            for segment in segments {
                let path = self.segments_path.join(&segment.file);
                let index = build_segment_index(segment.id, path, !segment.sealed)?;
                self.add_segment_index(index);
            }
        } else {
            let external_segments = self.external_segments.clone();
            for path in external_segments {
                let segment_id = self.next_external_segment_id;
                self.next_external_segment_id = self.next_external_segment_id.saturating_add(1);
                let index = build_segment_index(segment_id, path, false)?;
                self.add_segment_index(index);
            }
        }

        self.recompute_durable_stats();
        debug!(
            sealed_segments = self.sealed_indexes.len(),
            sources = self.durable_stats.len(),
            "storage indexes rebuilt",
        );
        Ok(())
    }

    fn add_segment_index(&mut self, index: SegmentIndex) {
        let segment_id = index.segment_id;

        for (source, message_indices) in &index.by_source {
            let pointers = self.source_index.entry(source.clone()).or_default();
            pointers.extend(
                message_indices
                    .iter()
                    .copied()
                    .map(|message_index| DurablePointer {
                        timestamp_nanos: index.messages[message_index].timestamp_nanos,
                        segment_id,
                        message_index,
                    }),
            );
        }

        self.sealed_indexes.insert(segment_id, index);

        for pointers in self.source_index.values_mut() {
            pointers.sort_by_key(|pointer| pointer.timestamp_nanos);
        }
    }

    fn recompute_durable_stats(&mut self) {
        self.durable_stats.clear();

        for (source, pointers) in &self.source_index {
            if pointers.is_empty() {
                continue;
            }

            let oldest_nanos = pointers.first().map(|pointer| pointer.timestamp_nanos);
            let latest_nanos = pointers.last().map(|pointer| pointer.timestamp_nanos);

            self.durable_stats.insert(
                source.clone(),
                DurableStats {
                    oldest: oldest_nanos.map(|value| {
                        from_nanos_with_id(
                            value,
                            self.timestamp_id_for_pointer(pointers.first().copied().unwrap())
                                .as_deref(),
                        )
                    }),
                    latest: latest_nanos.map(|value| {
                        from_nanos_with_id(
                            value,
                            self.timestamp_id_for_pointer(pointers.last().copied().unwrap())
                                .as_deref(),
                        )
                    }),
                    len: pointers.len() as u64,
                },
            );
        }

        for (source, indices) in &self.active_by_source {
            if indices.is_empty() {
                continue;
            }

            let stats = self
                .durable_stats
                .entry(source.clone())
                .or_insert(DurableStats {
                    oldest: None,
                    latest: None,
                    len: 0,
                });

            for index in indices {
                if let Some(record) = self.active_records.get(*index) {
                    stats.oldest = Some(match stats.oldest {
                        Some(existing) => existing.min(record.timestamp),
                        None => record.timestamp,
                    });
                    stats.latest = Some(match stats.latest {
                        Some(existing) => existing.max(record.timestamp),
                        None => record.timestamp,
                    });
                    stats.len = stats.len.saturating_add(1);
                }
            }
        }
    }

    fn timestamp_id_for_pointer(&self, pointer: DurablePointer) -> Option<String> {
        self.sealed_indexes
            .get(&pointer.segment_id)
            .and_then(|index| index.messages.get(pointer.message_index))
            .and_then(|message| message.timestamp_id.clone())
    }

    fn fetch_pointers(&self, pointers: &[DurablePointer]) -> Result<Vec<StreamRecord>> {
        let mut grouped: HashMap<u64, Vec<DurablePointer>> = HashMap::new();
        for pointer in pointers {
            grouped
                .entry(pointer.segment_id)
                .or_default()
                .push(*pointer);
        }

        let mut records = Vec::with_capacity(pointers.len());

        for (segment_id, segment_pointers) in grouped {
            let Some(segment_index) = self.sealed_indexes.get(&segment_id) else {
                continue;
            };

            let needs_summary = segment_pointers.iter().any(|pointer| {
                matches!(
                    segment_index
                        .messages
                        .get(pointer.message_index)
                        .map(|m| &m.locator),
                    Some(MessageLocator::Summary { .. })
                )
            });

            let bytes = if needs_summary {
                Some(fs::read(&segment_index.path)?)
            } else {
                None
            };
            let summary = match &bytes {
                Some(bytes) => Summary::read(bytes)?,
                None => None,
            };

            for pointer in segment_pointers {
                let Some(message) = segment_index.messages.get(pointer.message_index) else {
                    continue;
                };

                let timestamp =
                    from_nanos_with_id(message.timestamp_nanos, message.timestamp_id.as_deref());

                let payload = match &message.locator {
                    MessageLocator::InMemoryPayload(payload) => payload.clone(),
                    MessageLocator::Summary { chunk_index, entry } => {
                        let bytes = bytes.as_ref().ok_or(Error::BadDurableIndex)?;
                        let summary = summary.as_ref().ok_or(Error::BadDurableIndex)?;
                        let chunk = summary
                            .chunk_indexes
                            .get(*chunk_index)
                            .ok_or(Error::BadDurableIndex)?;
                        let fetched = summary.seek_message(bytes, chunk, entry)?;
                        Arc::from(fetched.data.into_owned().into_boxed_slice())
                    }
                };

                records.push(StreamRecord {
                    source: message.source.clone(),
                    effective_namespace: message.effective_namespace.clone(),
                    timestamp,
                    encoding: message.encoding.clone(),
                    payload,
                });
            }
        }

        Ok(records)
    }

    fn fetch_pointer(&self, pointer: DurablePointer) -> Result<StreamRecord> {
        self.fetch_pointers(&[pointer])
            .map(|mut records| records.pop())
            .and_then(|record| record.ok_or(Error::BadDurableIndex))
    }

    fn load_manifest(&self) -> Result<Manifest> {
        let path = self.manifest_path.clone().ok_or(Error::BackendClosed)?;
        let raw = fs::read_to_string(path)?;
        let manifest: Manifest = serde_json::from_str(&raw)?;
        if manifest.version != MANIFEST_VERSION {
            return Err(Error::UnsupportedManifestVersion {
                expected: MANIFEST_VERSION,
                found: manifest.version,
            });
        }
        Ok(manifest)
    }

    fn save_manifest(&self) -> Result<()> {
        let Some(path) = &self.manifest_path else {
            return Ok(());
        };
        let Some(manifest) = &self.manifest else {
            return Ok(());
        };

        fs::write(path, serde_json::to_string_pretty(manifest)?)?;
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        info!("storage shutdown started");
        if self.mode == OpenMode::ReadWrite {
            if let Some(mut active) = self.active_writer.take() {
                let sealed_segment_id = active.segment_id;
                let sealed_path = active.path.clone();
                active.writer.finish()?;
                debug!(
                    segment_id = sealed_segment_id,
                    "sealed final active segment on shutdown"
                );
                if let Some(manifest) = self.manifest.as_mut() {
                    if let Some(segment) = manifest
                        .segments
                        .iter_mut()
                        .find(|segment| segment.id == sealed_segment_id)
                    {
                        segment.sealed = true;
                    }
                    manifest.active_segment_id = None;
                }

                let sealed_index = build_segment_index(sealed_segment_id, sealed_path, false)?;
                self.add_segment_index(sealed_index);
                self.active_records.clear();
                self.active_by_source.clear();
                self.recompute_durable_stats();
                self.save_manifest()?;
            }
        }

        info!("storage shutdown complete");
        Ok(())
    }
}

fn discover_external_segments(root: &Path) -> Result<Vec<PathBuf>> {
    let mut segments = Vec::new();

    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "mcap")
        {
            segments.push(path);
        }
    }

    let nested_segments_dir = root.join("segments");
    if nested_segments_dir.exists() {
        for entry in fs::read_dir(nested_segments_dir)? {
            let path = entry?.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "mcap")
            {
                segments.push(path);
            }
        }
    }

    segments.sort();
    debug!(count = segments.len(), root = %root.display(), "discovered external MCAP segments");
    Ok(segments)
}

fn build_segment_index(
    segment_id: u64,
    path: PathBuf,
    allow_incomplete: bool,
) -> Result<SegmentIndex> {
    trace!(
        segment_id,
        path = %path.display(),
        allow_incomplete,
        "building segment index",
    );
    if !path.exists() {
        warn!(segment_id, path = %path.display(), "segment path missing while indexing");
        return Ok(SegmentIndex {
            segment_id,
            path,
            by_source: HashMap::new(),
            messages: Vec::new(),
        });
    }

    let bytes = fs::read(&path)?;

    if let Some(summary) = Summary::read(&bytes)? {
        let mut messages = Vec::new();
        let mut by_source: HashMap<String, Vec<usize>> = HashMap::new();

        for (chunk_index, chunk) in summary.chunk_indexes.iter().enumerate() {
            if let Ok(indexes) = summary.read_message_indexes(&bytes, chunk) {
                for (channel, entries) in indexes {
                    let (source, effective_namespace) =
                        source_from_topic_and_metadata(&channel.topic, &channel.metadata);
                    let source_key = source_key(&source);
                    let encoding = encoding_from_mcap(&channel.message_encoding);
                    let timestamp_id = timestamp_id_from_metadata(&channel.metadata);

                    for entry in entries {
                        let message_index = messages.len();
                        messages.push(SegmentIndexedMessage {
                            source: source.clone(),
                            effective_namespace: effective_namespace.clone(),
                            timestamp_nanos: entry.log_time,
                            timestamp_id: timestamp_id.clone(),
                            encoding: encoding.clone(),
                            locator: MessageLocator::Summary { chunk_index, entry },
                        });
                        by_source
                            .entry(source_key.clone())
                            .or_default()
                            .push(message_index);
                    }
                }
            }
        }

        for indices in by_source.values_mut() {
            indices.sort_by_key(|index| messages[*index].timestamp_nanos);
        }

        return Ok(SegmentIndex {
            segment_id,
            path,
            by_source,
            messages,
        });
    }

    // Fallback path for MCAP files without summary/index records.
    let mut stream = if allow_incomplete {
        MessageStream::new_with_options(&bytes, enum_set!(Options::IgnoreEndMagic))?
    } else {
        MessageStream::new(&bytes)?
    };

    let mut messages = Vec::new();
    let mut by_source: HashMap<String, Vec<usize>> = HashMap::new();

    for message in &mut stream {
        let message = message?;
        let (source, effective_namespace) =
            source_from_topic_and_metadata(&message.channel.topic, &message.channel.metadata);
        let source_key = source_key(&source);
        let timestamp_id = timestamp_id_from_metadata(&message.channel.metadata);

        let message_index = messages.len();
        messages.push(SegmentIndexedMessage {
            source,
            effective_namespace,
            timestamp_nanos: message.log_time,
            timestamp_id,
            encoding: encoding_from_mcap(&message.channel.message_encoding),
            locator: MessageLocator::InMemoryPayload(Arc::from(
                message.data.into_owned().into_boxed_slice(),
            )),
        });

        by_source.entry(source_key).or_default().push(message_index);
    }

    for indices in by_source.values_mut() {
        indices.sort_by_key(|index| messages[*index].timestamp_nanos);
    }

    trace!(
        segment_id,
        message_count = messages.len(),
        source_count = by_source.len(),
        "built segment index from linear message stream",
    );
    Ok(SegmentIndex {
        segment_id,
        path,
        by_source,
        messages,
    })
}

fn find_before_or_equal_pointer(
    pointers: &[DurablePointer],
    target: u64,
) -> Option<DurablePointer> {
    let upper = pointers.partition_point(|pointer| pointer.timestamp_nanos <= target);
    (upper > 0).then_some(pointers[upper - 1])
}

fn find_neighbor_pointers(pointers: &[DurablePointer], target: u64) -> [Option<DurablePointer>; 2] {
    let upper = pointers.partition_point(|pointer| pointer.timestamp_nanos < target);
    let before = (upper > 0).then_some(pointers[upper - 1]);
    let after = pointers.get(upper).copied();
    [before, after]
}

fn select_range_pointers(pointers: &[DurablePointer], start: u64, end: u64) -> Vec<DurablePointer> {
    let lower = pointers.partition_point(|pointer| pointer.timestamp_nanos < start);
    let upper = pointers.partition_point(|pointer| pointer.timestamp_nanos <= end);
    pointers[lower..upper].to_vec()
}
