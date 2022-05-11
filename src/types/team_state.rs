use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct TeamState {}
