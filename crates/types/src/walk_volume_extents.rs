use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkVolumeExtents {
    pub forward: f32,
    pub backward: f32,
    pub outward: f32,
    pub inward: f32,
    pub outward_rotation: f32,
    pub inward_rotation: f32,
}
