use std::time::SystemTime;

use mcap::records::system_time_to_nanos;
use ros_z::attachment::Attachment;
use zenoh::sample::Sample;

use crate::{RecordingError, Result};

#[derive(Debug)]
pub(crate) struct QueuedSample {
    pub topic_index: usize,
    pub sample: Sample,
    pub receive_time: SystemTime,
}

#[derive(Debug)]
pub(crate) struct McapSampleHeader {
    pub sequence: u32,
    pub log_time: u64,
    pub publish_time: u64,
}

pub(crate) fn sample_to_mcap_header(
    topic: &str,
    sample: &Sample,
    receive_time: SystemTime,
) -> Result<McapSampleHeader> {
    let attachment = sample
        .attachment()
        .ok_or(RecordingError::MissingSampleAttachment)
        .and_then(|raw| {
            Attachment::try_from(raw).map_err(RecordingError::SampleAttachmentDecode)
        })?;
    let sequence = u32::try_from(attachment.sequence_number).map_err(|_| {
        RecordingError::SequenceOutOfRange {
            topic: topic.to_string(),
            sequence: attachment.sequence_number,
        }
    })?;
    let log_time = sample
        .timestamp()
        .map(|timestamp| system_time_to_nanos(&timestamp.get_time().to_system_time()))
        .unwrap_or_else(|| system_time_to_nanos(&receive_time));

    Ok(McapSampleHeader {
        sequence,
        log_time,
        publish_time: u64::try_from(attachment.source_time().as_nanos()).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use ros_z::EndpointGlobalId;
    use ros_z::attachment::Attachment;
    use zenoh::sample::Sample;

    use super::sample_to_mcap_header;
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
    fn converts_raw_sample_metadata_to_mcap_message() {
        let sample = sample_with_attachment(42, "payload");

        let header = sample_to_mcap_header("/demo", &sample, UNIX_EPOCH + Duration::from_secs(10))
            .expect("sample converts");

        assert_eq!(header.sequence, 42);
        assert_eq!(header.publish_time, 123_456);
        assert_eq!(header.log_time, 10_000_000_000);
        assert_eq!(sample.payload().to_bytes().as_ref(), b"payload");
    }

    #[test]
    fn rejects_sample_without_ros_z_attachment() {
        let error =
            sample_to_mcap_header("/demo", &sample_without_attachment("payload"), UNIX_EPOCH)
                .expect_err("missing attachment must fail");

        assert!(matches!(error, RecordingError::MissingSampleAttachment));
    }

    #[test]
    fn rejects_negative_sequence() {
        let error =
            sample_to_mcap_header("/demo", &sample_with_attachment(-1, "payload"), UNIX_EPOCH)
                .expect_err("negative sequence must fail");

        assert!(matches!(
            error,
            RecordingError::SequenceOutOfRange { topic, sequence }
                if topic == "/demo" && sequence == -1
        ));
    }

    #[test]
    fn rejects_overflowing_sequence() {
        let overflowing_sequence = i64::from(u32::MAX) + 1;
        let error = sample_to_mcap_header(
            "/demo",
            &sample_with_attachment(overflowing_sequence, "payload"),
            UNIX_EPOCH,
        )
        .expect_err("overflowing sequence must fail");

        assert!(matches!(
            error,
            RecordingError::SequenceOutOfRange { topic, sequence }
                if topic == "/demo" && sequence == overflowing_sequence
        ));
    }

    #[test]
    fn accepts_sequence_zero() {
        let header =
            sample_to_mcap_header("/demo", &sample_with_attachment(0, "payload"), UNIX_EPOCH)
                .expect("sequence zero is valid");

        assert_eq!(header.sequence, 0);
    }
}
