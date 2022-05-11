use anyhow::Context;

use crate::{
    hardware::HardwareInterface,
    types::{CameraPosition, CycleInfo},
};

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
