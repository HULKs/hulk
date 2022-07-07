use anyhow::Context;
use types::{CameraPosition, CycleInfo};

use crate::hardware::HardwareInterface;

pub fn receive_image<Hardware>(
    hardware_interface: &Hardware,
    camera_position: CameraPosition,
) -> anyhow::Result<CycleInfo>
where
    Hardware: HardwareInterface,
{
    hardware_interface
        .produce_image_data(camera_position)
        .with_context(|| format!("Failed to receive image from camera {:?}", camera_position))
}
