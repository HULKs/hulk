use std::collections::BTreeMap;

use crate::{RecordingError, Result};

pub const RECORDER_NAME: &str = "ros-z-recording";
pub const METADATA_SCHEMA_VERSION: u32 = 1;
pub const SCHEMA_ENCODING: &str = "ros-z.schema.v1";
pub const MESSAGE_ENCODING: &str = "ros-z.cdr";

pub fn recording_metadata(
    requested_topics: &[String],
    resolved_topics_json: &str,
    timestamp_semantics_json: &str,
) -> BTreeMap<String, String> {
    let requested_topics = serde_json::to_string(requested_topics)
        .expect("serializing Vec<String> for MCAP metadata cannot fail");

    BTreeMap::from([
        ("recorder".to_string(), RECORDER_NAME.to_string()),
        (
            "recorder_version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ),
        (
            "metadata_schema_version".to_string(),
            METADATA_SCHEMA_VERSION.to_string(),
        ),
        ("schema_encoding".to_string(), SCHEMA_ENCODING.to_string()),
        ("message_encoding".to_string(), MESSAGE_ENCODING.to_string()),
        ("requested_topics".to_string(), requested_topics),
        (
            "resolved_topics".to_string(),
            resolved_topics_json.to_string(),
        ),
        (
            "timestamp_semantics".to_string(),
            timestamp_semantics_json.to_string(),
        ),
    ])
}

pub fn serialize_metadata_value<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(RecordingError::MetadataSerialize)
}

#[cfg(test)]
mod tests {
    use super::{MESSAGE_ENCODING, RECORDER_NAME, SCHEMA_ENCODING, recording_metadata};

    #[test]
    fn metadata_identifies_ros_z_recording_and_encodings_without_publisher_provenance() {
        let metadata = recording_metadata(
            &["chatter".to_string()],
            r#"[{"requested_topic":"chatter","topic":"/tools/chatter","type_name":"std_msgs::String","schema_hash":"RZHS02_hash","schema_encoding":"ros-z.schema.v1","message_encoding":"ros-z.cdr"}]"#,
            r#"{"log_time":"transport"}"#,
        );

        assert_eq!(metadata["recorder"], RECORDER_NAME);
        assert_eq!(metadata["schema_encoding"], SCHEMA_ENCODING);
        assert_eq!(metadata["message_encoding"], MESSAGE_ENCODING);
        assert_eq!(metadata["requested_topics"], r#"["chatter"]"#);
        assert!(!metadata["resolved_topics"].contains("publishers"));
    }
}
