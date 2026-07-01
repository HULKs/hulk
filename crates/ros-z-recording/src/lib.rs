//! MCAP recording backend for live ros-z topics.
//!
//! The crate records raw ros-z sample payloads together with the discovered
//! ros-z schema metadata needed to interpret them later. Topic names use normal
//! ros-z qualification rules through the node passed to [`RecordingSession`].

mod config;
mod error;
mod metadata;
mod runtime;
mod sample;
mod summary;
mod topic;
mod writer;

pub use config::RecordingConfig;
pub use error::{RecordingError, Result};
pub use metadata::{MESSAGE_ENCODING, METADATA_SCHEMA_VERSION, RECORDER_NAME, SCHEMA_ENCODING};
pub use runtime::RecordingSession;
pub use summary::{RecordingSummary, TopicSummary, format_system_time_utc};
