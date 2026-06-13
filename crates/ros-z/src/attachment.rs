use zenoh::bytes::ZBytes;
use zenoh_ext::{z_deserialize, z_serialize};

use crate::time::{Clock, Time};

pub use ros_z_protocol::entity::{ENDPOINT_GLOBAL_ID_SIZE, EndpointGlobalId};

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
            z_deserialize::<(i64, i64, [u8; ENDPOINT_GLOBAL_ID_SIZE])>(value)?;
        Ok(Attachment {
            sequence_number,
            source_timestamp,
            source_global_id: EndpointGlobalId::from(source_global_id),
        })
    }
}

impl From<Attachment> for ZBytes {
    fn from(value: Attachment) -> Self {
        z_serialize(&(
            value.sequence_number,
            value.source_timestamp,
            value.source_global_id.into_bytes(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_roundtrip_preserves_endpoint_global_id_bytes() {
        let bytes = [0xAB; ENDPOINT_GLOBAL_ID_SIZE];
        let endpoint_global_id = EndpointGlobalId::from(bytes);
        let attachment =
            Attachment::with_source_time(42, endpoint_global_id, Time::from_nanos(123_456));

        let encoded = ZBytes::from(attachment.clone());
        let decoded = Attachment::try_from(&encoded).unwrap();

        assert_eq!(decoded.sequence_number, 42);
        assert_eq!(decoded.source_timestamp, 123_456);
        assert_eq!(decoded.source_global_id, endpoint_global_id);
        assert_eq!(decoded.source_global_id.as_bytes(), &bytes);
    }
}
