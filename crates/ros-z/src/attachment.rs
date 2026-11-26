use zenoh::{bytes::ZBytes, sample::Sample};
use zenoh_ext::{z_deserialize, z_serialize};

use crate::error::{Error, WireError};
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

/// Decode required publication metadata without decoding the sample payload.
impl TryFrom<&Sample> for Attachment {
    type Error = Error;

    fn try_from(sample: &Sample) -> Result<Self, Self::Error> {
        let raw = sample
            .attachment()
            .ok_or(WireError::MissingSampleAttachment)?;

        Self::try_from(raw)
            .map_err(|source| Error::from(WireError::SampleAttachmentDecode { source }))
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
    fn sample_attachment_roundtrip_recovers_publication_metadata() {
        let bytes = [0xAB; ENDPOINT_GLOBAL_ID_SIZE];
        let endpoint_global_id = EndpointGlobalId::from(bytes);
        let attachment =
            Attachment::with_source_time(42, endpoint_global_id, Time::from_nanos(123_456));

        let encoded = ZBytes::from(attachment);
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let sample: Sample = zenoh::sample::SampleBuilder::put(key_expr, "payload")
            .attachment(encoded)
            .into();
        let decoded = Attachment::try_from(&sample).unwrap();

        assert_eq!(decoded.sequence_number, 42);
        assert_eq!(decoded.source_time(), Time::from_nanos(123_456));
        assert_eq!(decoded.source_global_id.as_bytes(), &bytes);
    }

    #[test]
    fn sample_attachment_conversion_rejects_missing_attachment() {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let sample: Sample = zenoh::sample::SampleBuilder::put(key_expr, "payload").into();
        assert!(matches!(
            Attachment::try_from(&sample),
            Err(Error::Wire(error)) if matches!(*error, WireError::MissingSampleAttachment)
        ));
    }

    #[test]
    fn sample_attachment_conversion_rejects_malformed_attachment() {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let sample: Sample = zenoh::sample::SampleBuilder::put(key_expr, "payload")
            .attachment(ZBytes::from(vec![0xFF]))
            .into();
        assert!(matches!(
            Attachment::try_from(&sample),
            Err(Error::Wire(error)) if matches!(*error, WireError::SampleAttachmentDecode { .. })
        ));
    }
}
