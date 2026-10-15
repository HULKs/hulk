//! Backend-independent plans over timestamped records.
mod live;
pub mod plan;
mod plan_observation;
mod projection;
pub(crate) mod typed;

use crate::{Error, Result};
use ros_z::{dynamic::Schema, time::Time};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::watch;

pub use live::{Live, LiveRetention};
pub use plan_observation::{
    PlanObservation, PlanSnapshot, PlanStatus, PlanValue, TypedPlanObservation, TypedPlanSnapshot,
};
pub use projection::{ProjectionError, TimeEncoding, TimeProjection};
pub use typed::{Connection, decode_record};

pub const MAX_ROWS: usize = 4096;
pub const MAX_PAYLOAD_BYTES: u64 = 32 * 1024 * 1024;

pub trait Backend: Send + Sync {
    fn topics(&self) -> TopicList;
    fn observe(&self, cursor: CursorHandle, plan: plan::QueryPlan) -> Result<PlanObservation>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Live,
    At(Time),
}
#[derive(Clone, Copy, Debug)]
pub struct CursorPosition {
    pub cursor: Cursor,
    pub revision: u64,
}

/// Clones share a cursor; constructing another handle creates an independent one.
#[derive(Clone, Debug)]
pub struct CursorHandle(watch::Sender<CursorPosition>);
impl CursorHandle {
    pub fn new(cursor: Cursor) -> Self {
        Self(
            watch::channel(CursorPosition {
                cursor,
                revision: 0,
            })
            .0,
        )
    }
    pub fn set(&self, cursor: Cursor) {
        self.0.send_if_modified(|position| {
            if position.cursor == cursor {
                return false;
            }
            position.cursor = cursor;
            position.revision = position.revision.wrapping_add(1);
            true
        });
    }
    /// Continuous playback preserves useful in-flight results. Rewinds and
    /// explicit seeks invalidate them immediately.
    pub fn advance(&self, time: Time) {
        self.0.send_if_modified(|position| {
            if position.cursor == Cursor::At(time) {
                return false;
            }
            if !matches!(position.cursor,Cursor::At(previous) if time>=previous) {
                position.revision = position.revision.wrapping_add(1);
            }
            position.cursor = Cursor::At(time);
            true
        });
    }
    pub fn position(&self) -> CursorPosition {
        *self.0.borrow()
    }
    pub fn subscribe(&self) -> watch::Receiver<CursorPosition> {
        self.0.subscribe()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicInfo {
    pub name: String,
    #[serde(with = "type_info_wire")]
    pub type_info: ros_z::TypeInfo,
    pub schema: Option<Schema>,
    pub error: Option<String>,
}
mod type_info_wire {
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: serde::Serializer>(
        info: &ros_z::TypeInfo,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        (&info.name, info.hash.0).serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ros_z::TypeInfo, D::Error> {
        let (name, hash) = <(String, [u8; 32])>::deserialize(deserializer)?;
        Ok(ros_z::TypeInfo::new(name, ros_z::SchemaHash(hash)))
    }
}
#[derive(Clone, Debug, Default)]
pub struct TopicSnapshot {
    pub topics: Vec<TopicInfo>,
    pub error: Option<String>,
    pub loading: bool,
}
#[derive(Clone)]
pub struct TopicList(pub(crate) watch::Receiver<Arc<TopicSnapshot>>);
impl TopicList {
    pub fn snapshot(&self) -> Arc<TopicSnapshot> {
        Arc::clone(&self.0.borrow())
    }
    pub fn subscribe_updates(&self) -> watch::Receiver<Arc<TopicSnapshot>> {
        self.0.clone()
    }
}

/// Shared payload storage. IDs are stable within a feed/recording instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawRecord {
    pub publication: Option<([u8; 16], i64)>,
    pub id: u64,
    pub time: Time,
    pub source_time: Time,
    pub log_time: Option<Time>,
    pub data: Arc<[u8]>,
}

/// Cardinality-preserving accessor for temporal selections used by interpolation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalMatch<T> {
    Exact(T),
    Before(T),
    Bracket { before: T, after: T },
}
impl<T> TemporalMatch<T> {
    pub fn last(&self) -> &T {
        match self {
            Self::Exact(value) | Self::Before(value) => value,
            Self::Bracket { after, .. } => after,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceCoverage {
    Complete,
    Live,
    RetentionLimited,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coverage {
    pub source: SourceCoverage,
    pub truncated: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MissingMatches {
    pub time: Time,
    pub inputs: Vec<usize>,
}

/// Half-open interval preserving results while source views stay unchanged.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CursorRange {
    pub start: Time,
    pub end: Option<Time>,
}
impl CursorRange {
    pub fn contains(self, cursor: Time) -> bool {
        self.start <= cursor && self.end.is_none_or(|end| cursor < end)
    }
}
pub fn query_error(error: impl std::fmt::Display) -> Error {
    Error::Query(error.to_string())
}
