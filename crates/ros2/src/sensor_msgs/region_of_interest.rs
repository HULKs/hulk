/// This message is used to specify a region of interest within an image.
///
/// When used to specify the ROI setting of the camera when the image was
/// taken, the height and width fields should either match the height and
/// width fields for the associated image; or height = width = 0
/// indicates that the full resolution image was captured.
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathIntrospect, PathSerialize, PathDeserialize,
)]
pub struct RegionOfInterest {
    /// Leftmost pixel of the ROI
    /// (0 if the ROI includes the left edge of the image)
    pub x_offset: u32,
    /// Topmost pixel of the ROI
    pub y_offset: u32,
    /// (0 if the ROI includes the top edge of the image)
    /// Height of ROI
    pub height: u32,
    /// Width of ROI
    pub width: u32,

    /// True if a distinct rectified ROI should be calculated from the "raw"
    /// ROI in this message. Typically this should be False if the full image
    /// is captured (ROI not used), and True if a subwindow is captured (ROI
    /// used).
    pub do_rectify: bool,
}
