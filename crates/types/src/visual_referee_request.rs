use std::default;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Copy, SerializeHierarchy, Serialize, Deserialize, Default)]
pub enum VisualRefereeRequest {
    #[default]
    No,
    Prepare,
    Observe,
}