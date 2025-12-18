/// This message contains an uncompressed image
/// (0, 0) is at top-left corner of image
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::std_msgs::header::Header;

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathIntrospect, PathSerialize, PathDeserialize,
)]
pub struct Image {
    /// Header timestamp should be acquisition time of image
    /// Header frame_id should be optical frame of camera
    /// If the frame_id here and the frame_id of the CameraInfo
    /// message associated with the image conflict
    /// the behavior is undefined
    pub header: Header,

    /// origin of frame should be optical center of cameara
    /// +x should point to the right in the image
    /// +y should point down in the image
    /// +z should point into to plane of the image
    ///
    /// image height, that is, number of rows
    pub height: u32,
    /// image width, that is, number of columns
    pub width: u32,

    /// The legal values for encoding are in file src/image_encodings.cpp
    /// If you want to standardize a new string format, join
    /// ros-users@lists.ros.org and send an email proposing a new encoding.
    /// Encoding of pixels -- channel meaning, ordering, size
    /// taken from the list of strings in include/sensor_msgs/image_encodings.hpp
    pub encoding: String,

    /// is this data bigendian?
    pub is_bigendian: u8,
    /// Full row length in bytes
    pub step: u32,
    /// actual matrix data, size is (step * rows)
    pub data: Vec<u8>,
}
