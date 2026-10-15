use std::{
    borrow::Cow,
    collections::{BTreeMap, VecDeque},
    fs::File,
    io::{Read, Seek},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use color_eyre::eyre::{Context, OptionExt, Result, bail, ensure};
use mcap::{
    Summary,
    records::{MessageHeader, Record},
    sans_io::{
        indexed_reader::{IndexedReader, IndexedReaderOptions},
        linear_reader::{LinearReadEvent, LinearReader, LinearReaderOptions},
        summary_reader::{SummaryReadEvent, SummaryReader},
    },
};
use ros_z::{TypeInfo, dynamic::Schema};

#[cfg(unix)]
use std::os::unix::fs::FileExt;
#[cfg(windows)]
use std::os::windows::fs::FileExt;

const MAX_CHUNK: usize = 64 * 1024 * 1024;
const CACHE_BYTES: usize = 32 * 1024 * 1024;

/// Each scanner has a private cursor. Positional reads keep parallel scans on
/// the original file descriptor without racing a shared seek position.
struct ChunkInput<'a> {
    file: &'a File,
    position: u64,
}

impl Read for ChunkInput<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        #[cfg(unix)]
        let read = self.file.read_at(bytes, self.position)?;
        #[cfg(windows)]
        let read = self.file.seek_read(bytes, self.position)?;
        self.position = self.position.checked_add(read as u64).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "chunk file offset overflow",
            )
        })?;
        Ok(read)
    }
}

pub struct Channel {
    pub topic: String,
    pub type_info: TypeInfo,
    pub schema: Schema,
}

/// Uncompressed records in their original file order. Index offsets point directly
/// into this buffer; only selected message payloads need separate ownership.
struct SnapshotChunk {
    bytes: Vec<u8>,
}

impl SnapshotChunk {
    fn message(&self, offset: u32) -> Result<(MessageHeader, Cow<'_, [u8]>)> {
        let start = offset as usize;
        let prefix = self
            .bytes
            .get(start..start + 9)
            .ok_or_eyre("invalid indexed record offset")?;
        let length = usize::try_from(u64::from_le_bytes(prefix[1..].try_into()?))?;
        let end = (start + 9)
            .checked_add(length)
            .ok_or_eyre("record length overflow")?;
        let body = self
            .bytes
            .get(start + 9..end)
            .ok_or_eyre("indexed record exceeds chunk")?;
        match mcap::parse_record(prefix[0], body)? {
            Record::Message { header, data } => Ok((header, data)),
            _ => bail!("indexed offset is not a message"),
        }
    }
}

pub struct Recording {
    file: File,
    summary: Summary,
    pub channels: BTreeMap<u16, Channel>,
    pub start: u64,
    pub end: u64,
    chunks: VecDeque<(u32, Arc<SnapshotChunk>)>,
    cached_bytes: usize,
    #[cfg(test)]
    pub decoded_chunks: usize,
}

impl Recording {
    pub fn open(path: &Path, namespace: &str) -> Result<Self> {
        let mut file = File::open(path).wrap_err_with(|| format!("opening {}", path.display()))?;
        let mut reader = SummaryReader::new();
        while let Some(event) = reader.next_event() {
            match event? {
                SummaryReadEvent::ReadRequest(need) => {
                    let read = file.read(reader.insert(need))?;
                    reader.notify_read(read);
                }
                SummaryReadEvent::SeekRequest(to) => reader.notify_seeked(file.seek(to)?),
            }
        }
        let mut summary = reader
            .finish()
            .ok_or_eyre("MCAP needs a summary and chunk indexes; recover/reindex it first")?;
        ensure!(
            !summary.chunk_indexes.is_empty(),
            "MCAP has no chunk indexes"
        );
        ensure!(
            u32::try_from(summary.chunk_indexes.len()).is_ok(),
            "too many MCAP chunks"
        );
        // Numeric chunk IDs must encode file order for deterministic timestamp ties.
        summary
            .chunk_indexes
            .sort_unstable_by_key(|chunk| chunk.chunk_start_offset);
        ensure!(
            summary
                .chunk_indexes
                .windows(2)
                .all(|pair| pair[0].chunk_start_offset < pair[1].chunk_start_offset),
            "duplicate chunk offsets"
        );
        let stats = summary
            .stats
            .as_ref()
            .ok_or_eyre("MCAP has no statistics")?;
        ensure!(stats.message_count > 0, "MCAP is empty");
        let (start, end) = (stats.message_start_time, stats.message_end_time);
        ensure!(
            start <= end && end < i64::MAX as u64,
            "unsupported recording timestamps"
        );
        let mut channels = BTreeMap::new();
        let mut topics = std::collections::BTreeSet::new();
        for (&id, channel) in &summary.channels {
            ensure!(
                channel.message_encoding == "ros-z-cdr",
                "{}: unsupported message encoding {}",
                channel.topic,
                channel.message_encoding
            );
            let schema = channel
                .schema
                .as_ref()
                .ok_or_eyre("channel has no schema")?;
            ensure!(
                schema.encoding == "ros-z-schema-json",
                "{}: unsupported schema encoding",
                channel.topic
            );
            let bundle: ros_z_schema::SchemaBundle = serde_json::from_slice(&schema.data)?;
            bundle.validate()?;
            let hash = ros_z_schema::compute_hash(&bundle)?;
            ensure!(
                channel.metadata.get("ros_z.schema_hash") == Some(&hash.to_hash_string()),
                "{}: schema hash mismatch",
                channel.topic
            );
            let name = channel
                .metadata
                .get("ros_z.type_name")
                .ok_or_eyre("missing ros_z.type_name")?;
            ensure!(
                name == &schema.name,
                "{}: inconsistent schema name",
                channel.topic
            );
            if let ros_z_schema::TypeDef::Named(root) = &bundle.root {
                ensure!(
                    root.as_str() == name,
                    "{}: schema root mismatch",
                    channel.topic
                );
            }
            let topic = ros_z::topic_name::qualify_topic_name(
                channel.topic.trim_start_matches('/'),
                namespace,
                "mcap_replay",
            )?;
            ensure!(
                topics.insert(topic.clone()),
                "duplicate recorded topic {topic}"
            );
            channels.insert(
                id,
                Channel {
                    topic,
                    type_info: TypeInfo::new(name, hash),
                    schema: Arc::new(bundle),
                },
            );
        }
        IndexedReader::new_with_options(
            &summary,
            IndexedReaderOptions::default().with_record_length_limit(MAX_CHUNK),
        )?;
        Ok(Self {
            file,
            summary,
            channels,
            start,
            end,
            chunks: VecDeque::new(),
            cached_bytes: 0,
            #[cfg(test)]
            decoded_chunks: 0,
        })
    }

    /// Visit borrowed record bodies without allocating or retaining message payloads.
    /// The library handles compression; offsets count all uncompressed records,
    /// including schemas and channels preceding messages.
    fn scan_chunk(
        &mut self,
        index: u32,
        cancel: &AtomicBool,
        visit: impl FnMut(u32, u8, &[u8]) -> Result<()>,
    ) -> Result<()> {
        let chunk = &self.summary.chunk_indexes[index as usize];
        Self::scan_chunk_at(&self.file, chunk, || cancel.load(Ordering::Relaxed), visit)?;
        #[cfg(test)]
        {
            self.decoded_chunks += 1;
        }
        Ok(())
    }

    fn scan_chunk_at(
        file: &File,
        chunk: &mcap::records::ChunkIndex,
        cancelled: impl Fn() -> bool,
        mut visit: impl FnMut(u32, u8, &[u8]) -> Result<()>,
    ) -> Result<()> {
        ensure!(
            chunk.chunk_length <= MAX_CHUNK as u64 && chunk.uncompressed_size <= MAX_CHUNK as u64,
            "chunk exceeds 64 MiB limit"
        );
        let expected = usize::try_from(chunk.uncompressed_size)?;
        let mut remaining = usize::try_from(chunk.chunk_length)?;
        let mut input = ChunkInput {
            file,
            position: chunk.chunk_start_offset,
        };
        let mut reader = LinearReader::new_with_options(
            LinearReaderOptions::default()
                .with_skip_start_magic(true)
                .with_skip_end_magic(true)
                .with_record_length_limit(MAX_CHUNK),
        );
        let mut offset = 0usize;
        while let Some(event) = reader.next_event() {
            ensure!(!cancelled(), "indexing cancelled");
            match event? {
                LinearReadEvent::ReadRequest(need) => {
                    let length = need.min(remaining);
                    input.read_exact(reader.insert(length))?;
                    reader.notify_read(length);
                    remaining -= length;
                }
                LinearReadEvent::Record { opcode, data } => {
                    let next = offset
                        .checked_add(9)
                        .and_then(|n| n.checked_add(data.len()))
                        .ok_or_eyre("chunk offset overflow")?;
                    ensure!(
                        next <= expected,
                        "records exceed indexed uncompressed chunk size"
                    );
                    visit(u32::try_from(offset)?, opcode, data)?;
                    offset = next;
                }
            }
        }
        ensure!(
            remaining == 0 && offset == expected,
            "chunk size does not match its index"
        );
        Ok(())
    }

    fn snapshot_chunk(&mut self, index: u32, cancel: &AtomicBool) -> Result<Arc<SnapshotChunk>> {
        if let Some(slot) = self.chunks.iter().position(|(cached, _)| *cached == index) {
            let chunk = self.chunks.remove(slot).expect("existing chunk");
            let result = Arc::clone(&chunk.1);
            self.chunks.push_back(chunk);
            return Ok(result);
        }
        let size = usize::try_from(self.summary.chunk_indexes[index as usize].uncompressed_size)?;
        ensure!(size <= MAX_CHUNK, "chunk exceeds 64 MiB limit");
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size)?;
        self.scan_chunk(index, cancel, |_, opcode, data| {
            bytes.push(opcode);
            bytes.extend_from_slice(&(data.len() as u64).to_le_bytes());
            bytes.extend_from_slice(data);
            Ok(())
        })?;
        let chunk = Arc::new(SnapshotChunk { bytes });
        let size = chunk.bytes.capacity();
        if size <= CACHE_BYTES {
            while self.cached_bytes + size > CACHE_BYTES {
                if let Some((_, old)) = self.chunks.pop_front() {
                    self.cached_bytes -= old.bytes.capacity();
                }
            }
            self.cached_bytes += size;
            self.chunks.push_back((index, Arc::clone(&chunk)));
        }
        Ok(chunk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_scan_honors_cancellation_without_moving_the_file_cursor() {
        let file =
            crate::tests::fixture_records(true, false, &[(0, 10, 100, 100), (0, 20, 200, 200)]);
        let recording = Recording::open(file.path(), "/replay").unwrap();
        #[cfg(unix)]
        let before = (&recording.file).stream_position().unwrap();
        let cancel = AtomicBool::new(false);
        let mut visits = 0;
        let result = Recording::scan_chunk_at(
            &recording.file,
            &recording.summary.chunk_indexes[0],
            || cancel.load(Ordering::Relaxed),
            |_, _, _| {
                visits += 1;
                cancel.store(true, Ordering::Relaxed);
                Ok(())
            },
        );
        assert!(result.unwrap_err().to_string().contains("cancelled"));
        assert_eq!(visits, 1);
        #[cfg(unix)]
        assert_eq!((&recording.file).stream_position().unwrap(), before);
    }
}
