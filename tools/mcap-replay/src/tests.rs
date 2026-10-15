use std::{borrow::Cow, collections::BTreeMap, sync::Arc, time::Duration};

use ros_z::{Message, message::WireEncoder, time::Time};

fn channels<T: Message>(topics: &[&str]) -> Vec<Arc<mcap::Channel<'static>>> {
    let bundle = T::schema();
    let hash = ros_z_schema::compute_hash(&bundle).unwrap();
    let schema = Arc::new(mcap::Schema {
        id: 1,
        name: T::type_name(),
        encoding: "ros-z-schema-json".into(),
        data: Cow::Owned(serde_json::to_vec(&bundle).unwrap()),
    });
    topics
        .iter()
        .enumerate()
        .map(|(id, topic)| {
            Arc::new(mcap::Channel {
                id: id as u16 + 1,
                topic: (*topic).into(),
                schema: Some(schema.clone()),
                message_encoding: "ros-z-cdr".into(),
                metadata: BTreeMap::from([
                    ("ros_z.type_name".into(), T::type_name()),
                    ("ros_z.schema_hash".into(), hash.to_hash_string()),
                ]),
            })
        })
        .collect()
}

pub(crate) fn fixture_records(
    message_indexes: bool,
    flush_each: bool,
    records: &[(usize, u64, u64, u64)],
) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    let channels = channels::<Time>(&["images", "detections"]);
    let mut writer = mcap::WriteOptions::new()
        .chunk_size(None)
        .emit_message_indexes(message_indexes)
        .create(file.reopen().unwrap())
        .unwrap();
    for (sequence, &(channel, stamp, source, log)) in records.iter().enumerate() {
        writer
            .write(&mcap::Message {
                channel: channels[channel].clone(),
                sequence: sequence as u32,
                log_time: log,
                publish_time: source,
                data: Cow::Owned(
                    <Time as Message>::Codec::serialize(
                        &Time::zero().saturating_add(Duration::from_nanos(stamp)),
                    )
                    .unwrap(),
                ),
            })
            .unwrap();
        if flush_each {
            writer.flush().unwrap();
        }
    }
    writer.finish().unwrap();
    file
}
