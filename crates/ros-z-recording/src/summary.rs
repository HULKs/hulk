use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingSummary {
    pub output_path: PathBuf,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub topics: Vec<TopicSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicSummary {
    pub topic: String,
    pub type_name: String,
    pub schema_hash: String,
    pub messages: u64,
    pub bytes: u64,
    pub drops: u64,
}

impl RecordingSummary {
    pub fn duration(&self) -> Duration {
        self.end_time
            .duration_since(self.start_time)
            .unwrap_or(Duration::ZERO)
    }

    pub fn topic_count(&self) -> usize {
        self.topics.len()
    }

    pub fn total_messages(&self) -> u64 {
        self.topics.iter().map(|topic| topic.messages).sum()
    }

    pub fn total_bytes(&self) -> u64 {
        self.topics.iter().map(|topic| topic.bytes).sum()
    }

    pub fn total_drops(&self) -> u64 {
        self.topics.iter().map(|topic| topic.drops).sum()
    }
}

pub fn format_system_time_utc(time: SystemTime) -> String {
    let timestamp: DateTime<Utc> = time.into();
    timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use super::{RecordingSummary, TopicSummary, format_system_time_utc};

    #[test]
    fn summary_totals_messages_bytes_and_drops_by_topic() {
        let summary = RecordingSummary {
            output_path: PathBuf::from("recording.mcap"),
            start_time: UNIX_EPOCH,
            end_time: UNIX_EPOCH + Duration::from_secs(3),
            topics: vec![
                TopicSummary {
                    topic: "/alpha".to_string(),
                    type_name: "test_msgs::Alpha".to_string(),
                    schema_hash: "RZHS02_alpha".to_string(),
                    messages: 2,
                    bytes: 10,
                    drops: 1,
                },
                TopicSummary {
                    topic: "/beta".to_string(),
                    type_name: "test_msgs::Beta".to_string(),
                    schema_hash: "RZHS02_beta".to_string(),
                    messages: 3,
                    bytes: 90,
                    drops: 0,
                },
            ],
        };

        assert_eq!(summary.duration(), Duration::from_secs(3));
        assert_eq!(summary.topic_count(), 2);
        assert_eq!(summary.total_messages(), 5);
        assert_eq!(summary.total_bytes(), 100);
        assert_eq!(summary.total_drops(), 1);
    }

    #[test]
    fn formats_system_time_as_utc_without_spaces() {
        let text = format_system_time_utc(UNIX_EPOCH + Duration::from_secs(3661));

        assert_eq!(text, "1970-01-01T01:01:01Z");
    }
}
