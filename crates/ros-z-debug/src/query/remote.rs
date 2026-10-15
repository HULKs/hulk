use bincode::Options;
use serde::{Deserialize, Serialize};

use super::*;

pub const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

pub fn query_key(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "hulk/mcap/query".into()
    } else {
        format!("hulk/mcap/{namespace}/query")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingInfo {
    pub instance: String,
    pub name: String,
    pub start: Time,
    pub end: Time,
    pub topics: Vec<TopicInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Request {
    Catalog,
    Plan {
        instance: String,
        cursor: Time,
        plan: plan::QueryPlan,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    RecordingChanged,
    Catalog(RecordingInfo),
    Error(String),
    PlanData(plan::Evaluation),
}

pub fn encode(response: &Response) -> Result<Vec<u8>> {
    bincode::DefaultOptions::new()
        .with_limit(MAX_RESPONSE_BYTES)
        .serialize(response)
        .map_err(query_error)
}

pub fn decode(bytes: &[u8]) -> Result<Response> {
    bincode::DefaultOptions::new()
        .with_limit(MAX_RESPONSE_BYTES)
        .deserialize(bytes)
        .map_err(query_error)
}
