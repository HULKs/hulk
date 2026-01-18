use serde::{Deserialize, Serialize};

use crate::geometry_msgs::{quaternion::Quaternion, vector3::Vector3};

/// This represents the transform between two coordinate frames in free space.
#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
}
