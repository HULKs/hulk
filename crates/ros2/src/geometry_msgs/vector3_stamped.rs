/// This represents a Vector3 with reference coordinate frame and timestamp

/// Note that this follows vector semantics with it always anchored at the origin,
/// so the rotational elements of a transform are the only parts applied when transforming.

use serde::{Deserialize, Serialize};

use crate::{geometry_msgs::vector3::Vector3, std_msgs::header::Header};

#[derive(Debug, Serialize, Deserialize)]
pub struct Vector3Stamped {
    pub header: Header,
    pub vector: Vector3,
}
