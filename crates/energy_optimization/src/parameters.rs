use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct CurrentMinimizerParameters {
    pub allowed_current: f32,
    pub allowed_current_upper_threshold: f32,
    pub optimization_speed: f32,
}
