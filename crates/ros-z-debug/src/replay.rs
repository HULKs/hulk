//! JSON control protocol for recorded-topic playback over Zenoh queries.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Absolute topic names and their current replay publisher identities.
pub type ReplaySources = BTreeMap<String, [u8; 16]>;

/// A query without a payload reads status. A command payload updates playback.
pub fn control_key(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "hulk/replay/control".into()
    } else {
        format!("hulk/replay/{namespace}/control")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayStatus {
    pub instance: String,
    pub recording: String,
    pub generation: u64,
    pub start: u64,
    pub end: u64,
    pub position: u64,
    pub playing: bool,
    pub sources: ReplaySources,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ReplayCommand {
    Play { instance: String },
    Pause { instance: String },
    Seek { instance: String, position: u64 },
}

impl ReplayCommand {
    pub fn instance(&self) -> &str {
        match self {
            Self::Play { instance } | Self::Pause { instance } | Self::Seek { instance, .. } => {
                instance
            }
        }
    }
}
