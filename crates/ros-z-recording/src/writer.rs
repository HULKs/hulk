use std::collections::BTreeMap;
use std::io::{Seek, Write};

use mcap::Writer;
use mcap::records::MessageHeader;
use mcap::write::Metadata;
use serde::Serialize;

use crate::metadata::{recording_metadata, serialize_metadata_value};
use crate::sample::RecordedSample;
use crate::topic::ResolvedTopic;
use crate::{MESSAGE_ENCODING, RecordingError, Result, SCHEMA_ENCODING};

pub struct McapWriterSink<W: Write + Seek> {
    writer: Writer<W>,
    channel_ids: Vec<u16>,
    topic_counts: Vec<WriterTopicSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterTopicSummary {
    pub messages: u64,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
struct ResolvedTopicMetadata<'a> {
    topic: &'a str,
    type_name: &'a str,
    schema_hash: &'a str,
    schema_encoding: &'static str,
    message_encoding: &'static str,
    publishers: &'a [crate::topic::PublisherInfo],
}

#[derive(Debug, Serialize)]
struct TimestampSemantics {
    log_time: &'static str,
    publish_time: &'static str,
}

impl<W: Write + Seek> McapWriterSink<W> {
    pub fn new(writer: W, topics: &[ResolvedTopic], requested_topics: &[String]) -> Result<Self> {
        let mut writer = Writer::new(writer).map_err(RecordingError::Mcap)?;
        let mut channel_ids = Vec::with_capacity(topics.len());

        for topic in topics {
            let schema_data = serde_json::to_vec(topic.schema().as_ref()).map_err(|source| {
                RecordingError::SchemaSerialize {
                    topic: topic.topic().to_string(),
                    source,
                }
            })?;
            let schema_id = writer
                .add_schema(topic.type_name(), SCHEMA_ENCODING, &schema_data)
                .map_err(RecordingError::Mcap)?;
            let channel_id = writer
                .add_channel(schema_id, topic.topic(), MESSAGE_ENCODING, &BTreeMap::new())
                .map_err(RecordingError::Mcap)?;
            channel_ids.push(channel_id);
        }

        let resolved_topics = topics
            .iter()
            .map(|topic| ResolvedTopicMetadata {
                topic: topic.topic(),
                type_name: topic.type_name(),
                schema_hash: topic.schema_hash(),
                schema_encoding: SCHEMA_ENCODING,
                message_encoding: MESSAGE_ENCODING,
                publishers: topic.publishers(),
            })
            .collect::<Vec<_>>();
        let resolved_topics_json = serialize_metadata_value(&resolved_topics)?;
        let timestamp_semantics_json = serialize_metadata_value(&TimestampSemantics {
            log_time: "Zenoh transport timestamp when present, otherwise recorder wall-clock receive time",
            publish_time: "ros-z source timestamp from sample attachment",
        })?;
        writer
            .write_metadata(&Metadata {
                name: "ros-z".to_string(),
                metadata: recording_metadata(
                    requested_topics,
                    &resolved_topics_json,
                    &timestamp_semantics_json,
                ),
            })
            .map_err(RecordingError::Mcap)?;

        Ok(Self {
            writer,
            channel_ids,
            topic_counts: vec![
                WriterTopicSummary {
                    messages: 0,
                    bytes: 0,
                };
                topics.len()
            ],
        })
    }

    pub fn write_sample(&mut self, sample: &RecordedSample) -> Result<()> {
        let topic_count = self.channel_ids.len();
        let channel_id = self.channel_ids.get(sample.topic_index).copied().ok_or(
            RecordingError::InvalidTopicIndex {
                topic_index: sample.topic_index,
                topic_count,
            },
        )?;
        let counts = self.topic_counts.get_mut(sample.topic_index).ok_or(
            RecordingError::InvalidTopicIndex {
                topic_index: sample.topic_index,
                topic_count,
            },
        )?;
        self.writer
            .write_to_known_channel(
                &MessageHeader {
                    channel_id,
                    sequence: sample.sequence,
                    log_time: sample.log_time,
                    publish_time: sample.publish_time,
                },
                &sample.payload,
            )
            .map_err(RecordingError::Mcap)?;
        counts.messages += 1;
        counts.bytes += sample.payload.len() as u64;
        Ok(())
    }

    pub fn finish(mut self) -> Result<Vec<WriterTopicSummary>> {
        self.writer.finish().map_err(RecordingError::Mcap)?;
        Ok(self.topic_counts)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ros_z::Message;

    use super::McapWriterSink;
    use crate::RecordingError;
    use crate::sample::RecordedSample;
    use crate::topic::{PublisherInfo, ResolvedTopic};

    fn resolved_topic(topic: &str) -> ResolvedTopic {
        ResolvedTopic::new(
            topic.to_string(),
            String::type_info(),
            Arc::new(String::schema()),
            vec![PublisherInfo {
                node: "/test/publisher".to_string(),
                schema_hash: String::schema_hash().to_hash_string(),
                endpoint_id: "01010101010101010101010101010101".to_string(),
            }],
        )
        .expect("test resolved topic")
    }

    #[test]
    fn writes_schema_channel_metadata_and_message() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::NamedTempFile::new()?;
        let file = std::fs::File::create(temp.path())?;
        let topics = vec![resolved_topic("/demo")];
        let mut sink = McapWriterSink::new(file, &topics, &["/demo".to_string()])?;

        sink.write_sample(&RecordedSample {
            topic_index: 0,
            sequence: 7,
            log_time: 10,
            publish_time: 9,
            payload: b"hello".to_vec(),
        })
        .expect("sample writes");
        let counts = sink.finish()?;

        assert_eq!(counts[0].messages, 1);
        assert_eq!(counts[0].bytes, 5);

        let bytes = std::fs::read(temp.path())?;
        let messages = mcap::MessageStream::new(&bytes)
            .expect("valid mcap")
            .collect::<Result<Vec<_>, _>>()
            .expect("messages read");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].channel.topic, "/demo");
        assert_eq!(
            messages[0].channel.message_encoding,
            crate::MESSAGE_ENCODING
        );
        assert_eq!(
            messages[0]
                .channel
                .schema
                .as_ref()
                .expect("schema")
                .encoding,
            crate::SCHEMA_ENCODING
        );
        Ok(())
    }

    #[test]
    fn rejects_sample_with_out_of_range_topic_index() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::NamedTempFile::new()?;
        let file = std::fs::File::create(temp.path())?;
        let topics = vec![resolved_topic("/demo")];
        let mut sink = McapWriterSink::new(file, &topics, &["/demo".to_string()])?;

        let error = sink
            .write_sample(&RecordedSample {
                topic_index: 1,
                sequence: 7,
                log_time: 10,
                publish_time: 9,
                payload: b"hello".to_vec(),
            })
            .expect_err("out-of-range topic index must fail");

        assert!(matches!(
            error,
            RecordingError::InvalidTopicIndex {
                topic_index: 1,
                topic_count: 1,
            }
        ));
        let counts = sink.finish()?;
        assert_eq!(counts[0].messages, 0);
        assert_eq!(counts[0].bytes, 0);
        Ok(())
    }
}
