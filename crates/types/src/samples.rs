use std::sync::Arc;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Samples {
    pub rate: u32,
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
