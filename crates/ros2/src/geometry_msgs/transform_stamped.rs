use serde::{Deserialize, Serialize};

use crate::{geometry_msgs::transform::Transform, std_msgs::header::Header};

/// This expresses a transform from coordinate frame header.frame_id
/// to the coordinate frame child_frame_id at the time of header.stamp
///
/// This message is mostly used by the
/// <a href="https://index.ros.org/p/tf2/">tf2</a> package.
/// See its documentation for more information.
///
/// The child_frame_id is necessary in addition to the frame_id
/// in the Header to communicate the full reference for the transform
/// in a self contained message.
#[repr(C)]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransformStamped {
    /// The frame id in the header is used as the reference frame of this transform.
    pub header: Header,

    /// The frame id of the child frame to which this transform points.
    pub child_frame_id: String,

    /// Translation and rotation in 3-dimensions of child_frame_id from header.frame_id.
    pub transform: Transform,
}
