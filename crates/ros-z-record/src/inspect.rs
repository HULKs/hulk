use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use color_eyre::eyre::{Context, Result, eyre};
use mcap::{
    MessageStream,
    read::{LinearReader, Summary},
    records::{Channel, Header, Record, SchemaHeader},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::ResolvedTopic;

#[derive(Debug, Clone, Serialize, Default)]
pub struct RosZInspection {
    pub session: Option<BTreeMap<String, String>>,
    pub requested_topics: Option<Vec<String>>,
    pub resolved_topics: Option<Vec<ResolvedTopic>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectedTopic {
    pub topic: String,
    pub requested_topic: Option<String>,
    pub type_name: Option<String>,
    pub schema_hash: Option<String>,
    pub schema_name: Option<String>,
    pub schema_encoding: Option<String>,
    pub message_encoding: Option<String>,
    pub channel_count: usize,
    pub message_count: u64,
    pub byte_count: u64,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectionReport {
    pub input: PathBuf,
    pub profile: Option<String>,
    pub library: Option<String>,
    pub summary_present: bool,
    pub schema_count: usize,
    pub channel_count: usize,
    pub attachment_count: usize,
    pub metadata_count: usize,
    pub message_count: u64,
    pub message_start_time: Option<u64>,
    pub message_end_time: Option<u64>,
    pub topics: Vec<InspectedTopic>,
    pub ros_z: RosZInspection,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct SchemaInfo {
    name: String,
    encoding: String,
}

#[derive(Debug, Clone)]
struct TopicAccumulator {
    requested_topic: Option<String>,
    type_name: Option<String>,
    schema_hash: Option<String>,
    schema_name: Option<String>,
    schema_encoding: Option<String>,
    message_encoding: Option<String>,
    channel_ids: BTreeSet<u16>,
    message_count: u64,
    byte_count: u64,
    source_ids: BTreeSet<String>,
}

impl TopicAccumulator {
    fn from_channel(channel: &Channel, schema: Option<&SchemaInfo>) -> Self {
        let mut topic = Self {
            requested_topic: channel.metadata.get("requested_topic").cloned(),
            type_name: channel.metadata.get("type_name").cloned(),
            schema_hash: channel.metadata.get("schema_hash").cloned(),
            schema_name: schema.map(|schema| schema.name.clone()),
            schema_encoding: schema.map(|schema| schema.encoding.clone()),
            message_encoding: Some(channel.message_encoding.clone()),
            channel_ids: BTreeSet::from([channel.id]),
            message_count: 0,
            byte_count: 0,
            source_ids: BTreeSet::new(),
        };

        if let Some(source_id) = channel.metadata.get("source_id") {
            topic.source_ids.insert(source_id.clone());
        }

        topic
    }

    fn update_from_channel(&mut self, channel: &mcap::Channel<'_>) {
        self.requested_topic = self
            .requested_topic
            .clone()
            .or_else(|| channel.metadata.get("requested_topic").cloned());
        self.type_name = self
            .type_name
            .clone()
            .or_else(|| channel.metadata.get("type_name").cloned())
            .or_else(|| channel.schema.as_ref().map(|schema| schema.name.clone()));
        self.schema_hash = self
            .schema_hash
            .clone()
            .or_else(|| channel.metadata.get("schema_hash").cloned());
        self.schema_name = self
            .schema_name
            .clone()
            .or_else(|| channel.schema.as_ref().map(|schema| schema.name.clone()));
        self.schema_encoding = self.schema_encoding.clone().or_else(|| {
            channel
                .schema
                .as_ref()
                .map(|schema| schema.encoding.clone())
        });
        self.message_encoding = self
            .message_encoding
            .clone()
            .or_else(|| Some(channel.message_encoding.clone()));
        self.channel_ids.insert(channel.id);
        if let Some(source_id) = channel.metadata.get("source_id") {
            self.source_ids.insert(source_id.clone());
        }
    }

    fn update_from_channel_record(&mut self, channel: &Channel, schema: Option<&SchemaInfo>) {
        self.requested_topic = self
            .requested_topic
            .clone()
            .or_else(|| channel.metadata.get("requested_topic").cloned());
        self.type_name = self
            .type_name
            .clone()
            .or_else(|| channel.metadata.get("type_name").cloned())
            .or_else(|| schema.map(|schema| schema.name.clone()));
        self.schema_hash = self
            .schema_hash
            .clone()
            .or_else(|| channel.metadata.get("schema_hash").cloned());
        self.schema_name = self
            .schema_name
            .clone()
            .or_else(|| schema.map(|schema| schema.name.clone()));
        self.schema_encoding = self
            .schema_encoding
            .clone()
            .or_else(|| schema.map(|schema| schema.encoding.clone()));
        self.message_encoding = self
            .message_encoding
            .clone()
            .or_else(|| Some(channel.message_encoding.clone()));
        self.channel_ids.insert(channel.id);
        if let Some(source_id) = channel.metadata.get("source_id") {
            self.source_ids.insert(source_id.clone());
        }
    }

    fn merge_resolved_topic(&mut self, topic: &ResolvedTopic) {
        self.requested_topic = self
            .requested_topic
            .clone()
            .or_else(|| Some(topic.requested_topic.clone()));
        self.type_name = self
            .type_name
            .clone()
            .or_else(|| Some(topic.type_name.clone()));
        self.schema_hash = self
            .schema_hash
            .clone()
            .or_else(|| Some(topic.schema_hash.clone()));
    }

    fn into_topic(self, topic: String) -> InspectedTopic {
        InspectedTopic {
            topic,
            requested_topic: self.requested_topic,
            type_name: self.type_name,
            schema_hash: self.schema_hash,
            schema_name: self.schema_name,
            schema_encoding: self.schema_encoding,
            message_encoding: self.message_encoding,
            channel_count: self.channel_ids.len(),
            message_count: self.message_count,
            byte_count: self.byte_count,
            source_ids: self.source_ids.into_iter().collect(),
        }
    }
}

pub fn inspect_file(path: impl AsRef<Path>) -> Result<InspectionReport> {
    let path = path.as_ref();
    let bytes =
        fs::read(path).with_context(|| format!("failed to read MCAP file {}", path.display()))?;

    let summary_present = Summary::read(&bytes)
        .context("failed to read MCAP summary")?
        .is_some();

    let mut warnings = Vec::new();
    let (
        header,
        _schemas,
        mut topics,
        metadata,
        schema_count,
        channel_count,
        metadata_count,
        attachment_count,
    ) = collect_static_records(&bytes)?;
    let ros_z = decode_ros_z_metadata(&metadata, &mut warnings);
    let (message_count, message_start_time, message_end_time) =
        collect_message_stats(&bytes, &mut topics)?;

    if let Some(resolved_topics) = ros_z.resolved_topics.as_ref() {
        for topic in resolved_topics {
            topics
                .entry(topic.qualified_topic.clone())
                .or_insert_with(|| TopicAccumulator {
                    requested_topic: None,
                    type_name: None,
                    schema_hash: None,
                    schema_name: None,
                    schema_encoding: None,
                    message_encoding: None,
                    channel_ids: BTreeSet::new(),
                    message_count: 0,
                    byte_count: 0,
                    source_ids: BTreeSet::new(),
                })
                .merge_resolved_topic(topic);
        }
    }

    Ok(InspectionReport {
        input: path.to_path_buf(),
        profile: non_empty(header.profile),
        library: non_empty(header.library),
        summary_present,
        schema_count,
        channel_count,
        attachment_count,
        metadata_count,
        message_count,
        message_start_time,
        message_end_time,
        topics: topics
            .into_iter()
            .map(|(topic, accumulator)| accumulator.into_topic(topic))
            .collect(),
        ros_z,
        warnings,
    })
}

type StaticRecords = (
    Header,
    BTreeMap<u16, SchemaInfo>,
    BTreeMap<String, TopicAccumulator>,
    BTreeMap<String, BTreeMap<String, String>>,
    usize,
    usize,
    usize,
    usize,
);

fn collect_static_records(bytes: &[u8]) -> Result<StaticRecords> {
    let mut header = None;
    let mut schemas = BTreeMap::new();
    let mut raw_channels = Vec::new();
    let mut metadata = BTreeMap::new();
    let mut metadata_count = 0usize;
    let mut attachment_count = 0usize;

    for record in LinearReader::new(bytes).context("failed to iterate MCAP records")? {
        match record.context("failed to parse MCAP record")? {
            Record::Header(found_header) => header = Some(found_header),
            Record::Schema {
                header: SchemaHeader { id, name, encoding },
                ..
            } => {
                schemas.insert(id, SchemaInfo { name, encoding });
            }
            Record::Channel(channel) => raw_channels.push(channel),
            Record::Metadata(found_metadata) => {
                metadata_count += 1;
                metadata.insert(found_metadata.name, found_metadata.metadata);
            }
            Record::Attachment { .. } => attachment_count += 1,
            _ => {}
        }
    }

    let header = header.ok_or_else(|| eyre!("MCAP file did not contain a header"))?;
    let schema_count = schemas.len();
    let channel_count = raw_channels.len();
    let mut topics = BTreeMap::new();
    for channel in &raw_channels {
        let schema = schemas.get(&channel.schema_id);
        topics
            .entry(channel.topic.clone())
            .and_modify(|topic: &mut TopicAccumulator| {
                topic.update_from_channel_record(channel, schema)
            })
            .or_insert_with(|| TopicAccumulator::from_channel(channel, schema));
    }

    Ok((
        header,
        schemas,
        topics,
        metadata,
        schema_count,
        channel_count,
        metadata_count,
        attachment_count,
    ))
}

fn collect_message_stats(
    bytes: &[u8],
    topics: &mut BTreeMap<String, TopicAccumulator>,
) -> Result<(u64, Option<u64>, Option<u64>)> {
    let mut message_count = 0u64;
    let mut start_time = None;
    let mut end_time = None;

    for message in MessageStream::new(bytes).context("failed to stream MCAP messages")? {
        let message = message.context("failed to decode MCAP message")?;
        message_count += 1;
        start_time = Some(start_time.map_or(message.log_time, |current: u64| {
            current.min(message.log_time)
        }));
        end_time = Some(end_time.map_or(message.log_time, |current: u64| {
            current.max(message.log_time)
        }));

        let entry = topics
            .entry(message.channel.topic.clone())
            .or_insert_with(|| TopicAccumulator {
                requested_topic: None,
                type_name: None,
                schema_hash: None,
                schema_name: None,
                schema_encoding: None,
                message_encoding: None,
                channel_ids: BTreeSet::new(),
                message_count: 0,
                byte_count: 0,
                source_ids: BTreeSet::new(),
            });
        entry.update_from_channel(&message.channel);
        entry.message_count += 1;
        entry.byte_count += u64::try_from(message.data.len()).unwrap_or(u64::MAX);
    }

    Ok((message_count, start_time, end_time))
}

fn decode_ros_z_metadata(
    metadata: &BTreeMap<String, BTreeMap<String, String>>,
    warnings: &mut Vec<String>,
) -> RosZInspection {
    RosZInspection {
        session: metadata.get("ros-z.session").cloned(),
        requested_topics: decode_metadata_json(metadata, "ros-z.request", "topics", warnings),
        resolved_topics: decode_metadata_json(
            metadata,
            "ros-z.resolved_topics",
            "topics",
            warnings,
        ),
    }
}

fn decode_metadata_json<T: DeserializeOwned>(
    metadata: &BTreeMap<String, BTreeMap<String, String>>,
    record_name: &str,
    field_name: &str,
    warnings: &mut Vec<String>,
) -> Option<T> {
    let value = metadata.get(record_name)?.get(field_name)?;

    match serde_json::from_str(value) {
        Ok(decoded) => Some(decoded),
        Err(error) => {
            warnings.push(format!(
                "failed to decode {record_name}.{field_name}: {error}"
            ));
            None
        }
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
