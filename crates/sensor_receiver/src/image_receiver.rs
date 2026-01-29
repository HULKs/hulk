use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, parameters::ImageReceiverInstance};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    instance: Parameter<ImageReceiverInstance, "image_receiver.$cycler_instance">,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<Image>,
    pub camera_info: MainOutput<Option<CameraInfo>>,
    pub cycle_time: MainOutput<CycleTime>,
}

impl ImageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl CameraInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let (image, camera_info) = match context.instance {
            ImageReceiverInstance::Rectified => {
                (context.hardware_interface.read_rectified_image()?, None)
            }
            ImageReceiverInstance::RightRectified => (
                context.hardware_interface.read_rectified_right_image()?,
                None,
            ),
            ImageReceiverInstance::RightRaw => (
                context.hardware_interface.read_image_right_raw()?,
                Some(
                    context
                        .hardware_interface
                        .read_image_right_raw_camera_info()?,
                ),
            ),
            ImageReceiverInstance::StereonetDepth => (
                context.hardware_interface.read_stereonet_depth_image()?,
                None,
            ),
            ImageReceiverInstance::StereonetVisual => (
                context.hardware_interface.read_stereonet_visual_image()?,
                None,
            ),
        };
        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        Ok(MainOutputs {
            image: image.into(),
            camera_info: camera_info.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
