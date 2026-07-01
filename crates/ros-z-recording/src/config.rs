use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::{RecordingError, Result};

const DEFAULT_WRITER_QUEUE_CAPACITY: usize = 1024;

#[derive(Debug, Clone)]
pub struct RecordingConfig {
    output_path: PathBuf,
    topics: Vec<String>,
    writer_queue_capacity: NonZeroUsize,
}

impl RecordingConfig {
    pub fn new(output_path: PathBuf, topics: Vec<String>) -> Result<Self> {
        Self::with_writer_queue_capacity(output_path, topics, default_writer_queue_capacity())
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }

    pub fn topics(&self) -> &[String] {
        &self.topics
    }

    pub(crate) fn with_writer_queue_capacity(
        output_path: PathBuf,
        topics: Vec<String>,
        writer_queue_capacity: NonZeroUsize,
    ) -> Result<Self> {
        if topics.is_empty() {
            return Err(RecordingError::EmptyTopicSelection);
        }

        Ok(Self {
            output_path,
            topics,
            writer_queue_capacity,
        })
    }

    #[cfg(test)]
    pub(crate) fn writer_queue_capacity(&self) -> NonZeroUsize {
        self.writer_queue_capacity
    }

    pub(crate) fn into_parts(self) -> (PathBuf, Vec<String>, NonZeroUsize) {
        (self.output_path, self.topics, self.writer_queue_capacity)
    }

    pub fn default_output_path(now: SystemTime) -> PathBuf {
        let timestamp: DateTime<Utc> = now.into();
        PathBuf::from(format!(
            "rosz-recording-{}.mcap",
            timestamp.format("%Y-%m-%dT%H-%M-%SZ")
        ))
    }
}

fn default_writer_queue_capacity() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_WRITER_QUEUE_CAPACITY)
        .expect("default writer queue capacity must be non-zero")
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use super::{DEFAULT_WRITER_QUEUE_CAPACITY, RecordingConfig};
    use crate::RecordingError;

    #[test]
    fn recording_config_rejects_empty_topics() {
        let error = RecordingConfig::new(PathBuf::from("out.mcap"), Vec::new())
            .expect_err("empty topic list must fail");

        assert!(matches!(error, RecordingError::EmptyTopicSelection));
    }

    #[test]
    fn recording_config_stores_exact_topics_and_default_queue_capacity() {
        let config = RecordingConfig::new(
            PathBuf::from("out.mcap"),
            vec!["/alpha".to_string(), "/beta".to_string()],
        )
        .expect("valid config");

        assert_eq!(config.output_path(), PathBuf::from("out.mcap"));
        assert_eq!(config.topics(), ["/alpha", "/beta"]);
        assert_eq!(
            config.writer_queue_capacity().get(),
            DEFAULT_WRITER_QUEUE_CAPACITY
        );
    }

    #[test]
    fn recording_config_keeps_writer_queue_capacity_crate_private_and_non_zero() {
        let config = RecordingConfig::with_writer_queue_capacity(
            PathBuf::from("out.mcap"),
            vec!["/alpha".to_string()],
            NonZeroUsize::new(1).expect("non-zero test capacity"),
        )
        .expect("valid config");

        assert_eq!(config.writer_queue_capacity().get(), 1);
    }

    #[test]
    fn default_output_path_uses_filename_safe_utc_timestamp() {
        let path = RecordingConfig::default_output_path(UNIX_EPOCH + Duration::from_secs(3661));

        assert_eq!(
            path,
            PathBuf::from("rosz-recording-1970-01-01T01-01-01Z.mcap")
        );
    }
}
