use serde::{Deserialize, Serialize};

use crate::std_msgs::header::Header;

#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
/// State of a set of joints at `header.stamp`.
///
/// Values in `name`, `position`, `velocity`, and `effort` correspond by index.
/// Numeric arrays may be empty when that state is not available.
pub struct JointState {
    /// Coordinate frame and sample timestamp.
    pub header: Header,
    /// Joint names in the same order as the numeric state arrays.
    pub name: Vec<String>,
    /// Joint positions in radians or meters.
    pub position: Vec<f64>,
    /// Joint velocities in radians per second or meters per second.
    pub velocity: Vec<f64>,
    /// Joint efforts in Newton meters or Newtons.
    pub effort: Vec<f64>,
}
