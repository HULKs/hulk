use color_eyre::{Result, eyre::eyre};
use image::RgbImage;
use ros2::sensor_msgs::image::Image as RosImage;

use crate::state::CameraFrame;

pub(super) fn decode_camera_frame(image: RosImage) -> Result<CameraFrame> {
    let rgb_image: RgbImage = image
        .try_into()
        .map_err(|error| eyre!("failed to decode camera image: {error}"))?;
    let width = rgb_image.width();
    let height = rgb_image.height();
    let rgba = rgb_image
        .into_vec()
        .chunks_exact(3)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
        .collect();

    Ok(CameraFrame {
        sequence: 0,
        width,
        height,
        rgba,
    })
}
