use std::{collections::BTreeMap, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct RecorderOptions {
    pub output: PathBuf,
    pub topics: Vec<String>,
    pub discovery_timeout: std::time::Duration,
    pub duration_limit: Option<std::time::Duration>,
    pub stats_interval: std::time::Duration,
    pub session_metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingStartup {
    pub output: PathBuf,
    pub requested_topics: Vec<String>,
    pub resolved_topics: Vec<ResolvedTopic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTopic {
    pub requested_topic: String,
    pub qualified_topic: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub schema_hash: String,
    pub schema_json: String,
    pub publishers: Vec<ResolvedPublisher>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPublisher {
    pub node_fqn: Option<String>,
    pub schema_hash: Option<String>,
    pub qos: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TopicStats {
    pub topic: String,
    pub messages: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StatsSnapshot {
    pub total_messages: u64,
    pub total_bytes: u64,
    pub topic_stats: Vec<TopicStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingReport {
    pub started_at: SystemTime,
    pub finished_at: SystemTime,
    pub total_messages: u64,
    pub total_bytes: u64,
    pub topic_stats: Vec<TopicStats>,
    pub silent_topics: Vec<String>,
}
