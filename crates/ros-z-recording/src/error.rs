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

    #[error("sample sequence {sequence} for topic {topic} cannot be represented in MCAP")]
    SequenceOutOfRange { topic: String, sequence: i64 },

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

    #[error("failed to discover schema for requested topic {topic}")]
    SchemaDiscovery {
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
            finalize: Box::new(RecordingError::OutputAlreadyExists {
                path: PathBuf::from("finalize.mcap"),
            }),
        };

        let message = error.to_string();

        assert!(
            message.contains("receive.mcap"),
            "combined error should include receive failure: {message}"
        );
        assert!(
            message.contains("finalize.mcap"),
            "combined error should include finalize failure: {message}"
        );
        assert!(
            error.source().is_some(),
            "combined error should expose at least the receive error as source"
        );
    }
}
