use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, RecordingError>;

#[derive(Debug, thiserror::Error)]
pub enum RecordingError {
    #[error("at least one topic must be requested")]
    EmptyTopicSelection,

    #[error("output path already exists: {}", path.display())]
    OutputAlreadyExists { path: PathBuf },

    #[error("failed to create output file: {}", path.display())]
    OutputCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize recorder metadata")]
    MetadataSerialize(#[source] serde_json::Error),

    #[error("sample is missing ros-z attachment metadata")]
    MissingSampleAttachment,

    #[error("failed to decode ros-z sample attachment")]
    SampleAttachmentDecode(#[source] zenoh::Error),

    #[error("failed to subscribe to topic {topic}")]
    Subscribe {
        topic: String,
        #[source]
        source: ros_z::Error,
    },

    #[error("recording receive task failed for topic {topic}")]
    ReceiveTask {
        topic: String,
        #[source]
        source: Box<RecordingError>,
    },

    #[error("failed to receive sample for topic {topic}")]
    Receive {
        topic: String,
        #[source]
        source: ros_z::Error,
    },

    #[error("recording finalized after a receive error")]
    RecordingStoppedAfterReceiveError {
        #[source]
        source: Box<RecordingError>,
        summary: Box<crate::RecordingSummary>,
    },

    #[error("recording receive failed ({receive}) and finalization failed ({finalize})")]
    ReceiveAndFinalize {
        #[source]
        receive: Box<RecordingError>,
        finalize: Box<RecordingError>,
    },

    #[error("recording task join failed")]
    Join(#[source] tokio::task::JoinError),

    #[error("failed to serialize topic schema for {topic}")]
    SchemaSerialize {
        topic: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to write MCAP data")]
    Mcap(#[source] mcap::McapError),

    #[error("sample topic index {topic_index} is out of range for {topic_count} writer topics")]
    InvalidTopicIndex {
        topic_index: usize,
        topic_count: usize,
    },

    #[error("topic has no visible publishers: {topic}")]
    TopicWithoutPublishers { topic: String },

    #[error("topic has publishers with conflicting type metadata: {topic}: {details:?}")]
    ConflictingTopicTypes { topic: String, details: Vec<String> },

    #[error("failed to build schema client for {service}")]
    SchemaClient {
        service: String,
        #[source]
        source: ros_z::Error,
    },

    #[error("failed to call schema service {service}")]
    SchemaCall {
        service: String,
        #[source]
        source: ros_z::Error,
    },

    #[error("schema service {service} rejected schema request: {reason}")]
    SchemaRejected { service: String, reason: String },

    #[error("schema response for topic {topic} is invalid")]
    SchemaResponse {
        topic: String,
        #[source]
        source: ros_z::dynamic::DynamicError,
    },
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;
    use std::path::PathBuf;

    use super::RecordingError;

    #[test]
    fn receive_and_finalize_display_keeps_both_errors_and_exposes_source() {
        let error = RecordingError::ReceiveAndFinalize {
            receive: Box::new(RecordingError::OutputAlreadyExists {
                path: PathBuf::from("receive.mcap"),
            }),
            finalize: Box::new(RecordingError::TopicWithoutPublishers {
                topic: "/missing".to_string(),
            }),
        };

        let message = error.to_string();

        assert!(
            message.contains("receive.mcap"),
            "combined error should include receive failure: {message}"
        );
        assert!(
            message.contains("/missing"),
            "combined error should include finalize failure: {message}"
        );
        assert!(
            error.source().is_some(),
            "combined error should expose at least the receive error as source"
        );
    }

    #[test]
    fn conflicting_topic_types_display_includes_conflict_details() {
        let error = RecordingError::ConflictingTopicTypes {
            topic: "/demo".to_string(),
            details: vec![
                "/node_a has test_msgs::Alpha [RZHS02_a]".to_string(),
                "/node_b has test_msgs::Beta [RZHS02_b]".to_string(),
            ],
        };

        let message = error.to_string();

        assert!(message.contains("/demo"));
        assert!(
            message.contains("/node_a has test_msgs::Alpha [RZHS02_a]"),
            "conflict display should include first detail: {message}"
        );
        assert!(
            message.contains("/node_b has test_msgs::Beta [RZHS02_b]"),
            "conflict display should include second detail: {message}"
        );
    }
}
