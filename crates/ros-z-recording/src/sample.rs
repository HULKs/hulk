use std::time::SystemTime;

use mcap::records::system_time_to_nanos;
use ros_z::attachment::Attachment;
use zenoh::sample::Sample;

use crate::{RecordingError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedSample {
    pub topic_index: usize,
    pub sequence: u32,
    pub log_time: u64,
    pub publish_time: u64,
    pub payload: Vec<u8>,
}

pub fn sample_to_record(
    topic_index: usize,
    sample: Sample,
    receive_time: SystemTime,
) -> Result<RecordedSample> {
    let attachment = sample
        .attachment()
        .ok_or(RecordingError::MissingSampleAttachment)
        .and_then(|raw| {
            Attachment::try_from(raw).map_err(RecordingError::SampleAttachmentDecode)
        })?;
    let log_time = sample
        .timestamp()
        .map(|timestamp| system_time_to_nanos(&timestamp.get_time().to_system_time()))
        .unwrap_or_else(|| system_time_to_nanos(&receive_time));
    let payload = sample.payload().to_bytes().to_vec();

    Ok(RecordedSample {
        topic_index,
        // MCAP stores sequences as u32; non-representable ros-z i64 values use 0.
        sequence: u32::try_from(attachment.sequence_number).unwrap_or(0),
        log_time,
        publish_time: u64::try_from(attachment.source_time().as_nanos()).unwrap_or(0),
        payload,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use ros_z::EndpointGlobalId;
    use ros_z::attachment::Attachment;
    use zenoh::sample::Sample;

    use super::sample_to_record;
    use crate::RecordingError;

    fn sample_with_attachment(sequence: i64, payload: &str) -> Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let attachment = Attachment::with_source_time(
            sequence,
            EndpointGlobalId::from([7; 16]),
            ros_z::time::Time::from_nanos(123_456),
        );
        zenoh::sample::SampleBuilder::put(key_expr, payload)
            .attachment(attachment)
            .into()
    }

    fn sample_without_attachment(payload: &str) -> Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        zenoh::sample::SampleBuilder::put(key_expr, payload).into()
    }

    #[test]
    fn converts_raw_sample_to_recorded_sample() {
        let recorded = sample_to_record(
            3,
            sample_with_attachment(42, "payload"),
            UNIX_EPOCH + Duration::from_secs(10),
        )
        .expect("sample converts");

        assert_eq!(recorded.topic_index, 3);
        assert_eq!(recorded.sequence, 42);
        assert_eq!(recorded.publish_time, 123_456);
        assert_eq!(recorded.log_time, 10_000_000_000);
        assert_eq!(recorded.payload, b"payload");
    }

    #[test]
    fn rejects_sample_without_ros_z_attachment() {
        let error = sample_to_record(0, sample_without_attachment("payload"), UNIX_EPOCH)
            .expect_err("missing attachment must fail");

        assert!(matches!(error, RecordingError::MissingSampleAttachment));
    }

    #[test]
    fn maps_negative_sequence_to_zero() {
        let recorded = sample_to_record(0, sample_with_attachment(-1, "payload"), UNIX_EPOCH)
            .expect("sample converts");

        assert_eq!(recorded.sequence, 0);
    }

    #[test]
    fn maps_overflowing_sequence_to_zero() {
        let overflowing_sequence = i64::from(u32::MAX) + 1;
        let recorded = sample_to_record(
            0,
            sample_with_attachment(overflowing_sequence, "payload"),
            UNIX_EPOCH,
        )
        .expect("sample converts");

        assert_eq!(recorded.sequence, 0);
    }
}
