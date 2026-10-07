use std::{
    collections::{BTreeMap, VecDeque},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};

use color_eyre::eyre::{Context, OptionExt, Result, ensure};
use mcap::{
    Summary,
    records::MessageHeader,
    sans_io::{
        indexed_reader::{IndexedReadEvent, IndexedReader, IndexedReaderOptions, ReadOrder},
        summary_reader::{SummaryReadEvent, SummaryReader},
    },
};
use ros_z::{TypeInfo, dynamic::Schema};

const MAX_CHUNK: usize = 64 * 1024 * 1024;
const CACHE_BYTES: usize = 32 * 1024 * 1024;

pub struct Channel {
    pub topic: String,
    pub type_info: TypeInfo,
    pub schema: Schema,
}

#[derive(Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub data: zenoh::bytes::ZBytes,
}

struct SnapshotChunk {
    channels: BTreeMap<u16, Vec<Message>>,
    bytes: usize,
}

pub struct Recording {
    file: File,
    summary: Summary,
    pub channels: BTreeMap<u16, Channel>,
    pub start: u64,
    pub end: u64,
    // Per-channel chunk candidates, descending by end time and file offset.
    channel_chunks: BTreeMap<u16, Vec<usize>>,
    chunks: VecDeque<(usize, Arc<SnapshotChunk>)>,
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
        let summary = reader
            .finish()
            .ok_or_eyre("MCAP needs a summary and chunk indexes; recover/reindex it first")?;
        ensure!(
            !summary.chunk_indexes.is_empty(),
            "MCAP has no chunk indexes"
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
            // Both absolute and relative recorded topics are rooted under the replay namespace.
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
        // Validate chunk bounds before advertising anything.
        IndexedReader::new_with_options(&summary, options())?;
        let channel_chunks = channels
            .keys()
            .map(|&id| {
                let mut chunks: Vec<_> = summary
                    .chunk_indexes
                    .iter()
                    .enumerate()
                    .filter(|(_, chunk)| {
                        chunk.message_index_offsets.is_empty()
                            || chunk.message_index_offsets.contains_key(&id)
                    })
                    .map(|(index, _)| index)
                    .collect();
                chunks.sort_by_key(|&index| {
                    let chunk = &summary.chunk_indexes[index];
                    std::cmp::Reverse((chunk.message_end_time, chunk.chunk_start_offset))
                });
                (id, chunks)
            })
            .collect();
        Ok(Self {
            file,
            summary,
            channels,
            start,
            end,
            channel_chunks,
            chunks: VecDeque::new(),
            cached_bytes: 0,
            #[cfg(test)]
            decoded_chunks: 0,
        })
    }

    pub fn forward(&self, after: u64) -> Result<IndexedReader> {
        Ok(IndexedReader::new_with_options(
            &self.summary,
            options()
                .with_order(ReadOrder::LogTime)
                .log_time_on_or_after(after.saturating_add(1)),
        )?)
    }

    /// Latest value per channel, inclusive of the cursor. Decode each chunk once
    /// for all topics and binary-search its sorted messages on subsequent seeks.
    pub fn snapshot(&mut self, position: u64) -> Result<Vec<Message>> {
        let channels: Vec<_> = self.channels.keys().copied().collect();
        let mut messages = Vec::new();
        for channel in channels {
            let mut best: Option<((u64, u64), Message)> = None;
            for candidate in 0..self.channel_chunks[&channel].len() {
                let index = self.channel_chunks[&channel][candidate];
                let chunk = &self.summary.chunk_indexes[index];
                if best
                    .as_ref()
                    .is_some_and(|((time, _), _)| chunk.message_end_time < *time)
                {
                    break;
                }
                if chunk.message_start_time > position {
                    continue;
                }
                let offset = chunk.chunk_start_offset;
                let chunk = self.snapshot_chunk(index)?;
                let Some(samples) = chunk.channels.get(&channel) else {
                    continue;
                };
                let end = samples.partition_point(|message| message.header.log_time <= position);
                if let Some(message) = end.checked_sub(1).map(|index| &samples[index]) {
                    let key = (message.header.log_time, offset);
                    if best.as_ref().is_none_or(|(previous, _)| key > *previous) {
                        best = Some((key, message.clone()));
                    }
                }
            }
            if let Some((_, message)) = best {
                messages.push(message);
            }
        }
        messages.sort_by_key(|message| (message.header.log_time, message.header.channel_id));
        Ok(messages)
    }

    fn snapshot_chunk(&mut self, index: usize) -> Result<Arc<SnapshotChunk>> {
        if let Some(slot) = self.chunks.iter().position(|(cached, _)| *cached == index) {
            let chunk = self.chunks.remove(slot).expect("existing chunk");
            let result = Arc::clone(&chunk.1);
            self.chunks.push_back(chunk);
            return Ok(result);
        }
        let summary = Summary {
            chunk_indexes: vec![self.summary.chunk_indexes[index].clone()],
            ..Default::default()
        };
        // File order plus a stable sort preserves last-in-file tie breaking.
        let mut reader =
            IndexedReader::new_with_options(&summary, options().with_order(ReadOrder::File))?;
        let mut chunk = SnapshotChunk {
            channels: BTreeMap::new(),
            bytes: 0,
        };
        while let Some(message) = self.next(&mut reader)? {
            chunk.bytes += message.data.len();
            chunk
                .channels
                .entry(message.header.channel_id)
                .or_default()
                .push(message);
        }
        for messages in chunk.channels.values_mut() {
            messages.sort_by_key(|message| message.header.log_time);
            chunk.bytes += messages.capacity() * std::mem::size_of::<Message>();
        }
        #[cfg(test)]
        {
            self.decoded_chunks += 1;
        }
        let chunk = Arc::new(chunk);
        if chunk.bytes <= CACHE_BYTES {
            while self.cached_bytes + chunk.bytes > CACHE_BYTES {
                if let Some((_, chunk)) = self.chunks.pop_front() {
                    self.cached_bytes -= chunk.bytes;
                }
            }
            self.cached_bytes += chunk.bytes;
            self.chunks.push_back((index, Arc::clone(&chunk)));
        }
        Ok(chunk)
    }

    pub fn next(&mut self, reader: &mut IndexedReader) -> Result<Option<Message>> {
        while let Some(event) = reader.next_event() {
            match event? {
                IndexedReadEvent::Message { header, data } => {
                    ensure!(
                        header.publish_time <= i64::MAX as u64,
                        "source timestamp exceeds ROS-Z time range"
                    );
                    ensure!(
                        self.channels.contains_key(&header.channel_id),
                        "message references an unknown channel"
                    );
                    return Ok(Some(Message {
                        header,
                        data: data.to_vec().into(),
                    }));
                }
                IndexedReadEvent::ReadChunkRequest { offset, length } => {
                    ensure!(length <= MAX_CHUNK, "chunk exceeds 64 MiB limit");
                    let mut bytes = vec![0; length];
                    self.file.seek(SeekFrom::Start(offset))?;
                    self.file.read_exact(&mut bytes)?;
                    reader.insert_chunk_record_data(offset, &bytes)?;
                }
            }
        }
        Ok(None)
    }
}

fn options() -> IndexedReaderOptions {
    IndexedReaderOptions::default().with_record_length_limit(MAX_CHUNK)
}
