use zenoh::bytes::ZBytes;
use zenoh_ext::{z_deserialize, z_serialize};

use crate::time::{Clock, Time};

const RMW_GID_STORAGE_SIZE: usize = 16;

pub type EndpointGlobalId = [u8; RMW_GID_STORAGE_SIZE];

#[derive(Clone, Hash)]
pub struct Attachment {
    pub sequence_number: i64,
    pub source_timestamp: i64,
    pub source_global_id: EndpointGlobalId,
}

impl Attachment {
    pub fn new(sequence_number: i64, source_global_id: EndpointGlobalId) -> Self {
        Self::with_source_time(
            sequence_number,
            source_global_id,
            Time::from_wallclock(std::time::SystemTime::now()),
        )
    }

    pub fn with_clock(
        sequence_number: i64,
        source_global_id: EndpointGlobalId,
        clock: &Clock,
    ) -> Self {
        Self::with_source_time(sequence_number, source_global_id, clock.now())
    }

    pub fn with_source_time(
        sequence_number: i64,
        source_global_id: EndpointGlobalId,
        source_time: Time,
    ) -> Self {
        Self {
            sequence_number,
            source_timestamp: source_time.as_nanos(),
            source_global_id,
        }
    }

    pub fn source_time(&self) -> Time {
        Time::from_nanos(self.source_timestamp)
    }
}

impl TryFrom<&ZBytes> for Attachment {
    type Error = zenoh::Error;
    fn try_from(value: &ZBytes) -> Result<Self, Self::Error> {
        let (sequence_number, source_timestamp, source_global_id) =
            z_deserialize::<(i64, i64, EndpointGlobalId)>(value)?;
        Ok(Attachment {
            sequence_number,
            source_timestamp,
            source_global_id,
        })
    }
}

impl From<Attachment> for ZBytes {
    fn from(value: Attachment) -> Self {
        z_serialize(&(
            value.sequence_number,
            value.source_timestamp,
            value.source_global_id,
        ))
    }
}
