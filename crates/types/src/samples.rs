use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct Samples {
    pub rate: u32,
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
