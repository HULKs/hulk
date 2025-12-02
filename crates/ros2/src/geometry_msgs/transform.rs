/// This represents the transform between two coordinate frames in free space.
use serde::{Deserialize, Serialize};

use crate::geometry_msgs::{quaternion::Quaternion, vector3::Vector3};

#[repr(C)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
}
