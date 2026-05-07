use std::sync::Arc;

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Message,
)]
pub struct Samples {
    pub rate: u32,
    #[path_serde(leaf)]
    pub channels_of_samples: Arc<Vec<Vec<f32>>>,
}
