//! Timestamped message wrapper.
//!
//! All received messages are wrapped in [`Message<T>`] which pairs the
//! deserialized payload with its Zenoh timestamp.

use serde::{Deserialize, Serialize};

use crate::Timestamp;

/// A received message with its Zenoh timestamp.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message<T> {
    pub timestamp: Timestamp,
    pub payload: T,
}
