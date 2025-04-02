use std::collections::{BTreeMap, HashMap};
use std::io::{Seek, Write};
use std::time::SystemTime;

use color_eyre::eyre::Result;
use mcap::records::{system_time_to_nanos, MessageHeader};
use mcap::write::Metadata;
use mcap::{Attachment, McapError, Writer};
use serde::Serialize;

use crate::serializer::Serializer;

type ChannelId = u16;
pub struct McapConverter<W: Write + Seek> {
    writer: Writer<W>,
    channel_mapping: BTreeMap<String, ChannelId>,
}

impl<W: Write + Seek> McapConverter<W> {
    pub fn from_writer(writer: W) -> Result<Self, McapError> {
        Ok(Self {
            writer: Writer::new(writer)?,
            channel_mapping: Default::default(),
        })
    }

    fn create_new_channel(&mut self, topic: String) -> Result<ChannelId, McapError> {
        let channel_id = self
            .writer
            .add_channel(0, &topic, "MessagePack", &Default::default())?;
        self.channel_mapping.insert(topic, channel_id);

        Ok(channel_id)
    }

    pub fn add_to_mcap(
        &mut self,
        topic: String,
        data: &[u8],
        sequence_number: u32,
        system_time: SystemTime,
    ) -> Result<(), McapError> {
        let channel_id = match self.channel_mapping.get(&topic).copied() {
            Some(channel_id) => channel_id,
            None => self.create_new_channel(topic)?,
        };
        let log_time = system_time_to_nanos(&system_time);

        self.writer.write_to_known_channel(
            &MessageHeader {
                channel_id,
                sequence: sequence_number,
                log_time,
                publish_time: log_time,
            },
            data,
        )?;

        Ok(())
    }

    pub fn finish(mut self) -> Result<(), McapError> {
        self.writer.finish()
    }

    pub fn write_metadata(&mut self, metadata: Metadata) -> Result<(), McapError> {
        self.writer.write_metadata(&metadata)
    }

    pub fn attach(&mut self, attachment: Attachment) -> Result<(), McapError> {
        self.writer.attach(&attachment)
    }
}

pub fn database_to_values<D: Serialize>(database: &D) -> Result<HashMap<String, Vec<u8>>> {
    let mut serializer = Serializer::default();
    database.serialize(&mut serializer)?;

    Ok(serializer.finish())
}
