use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, ycbcr422_image::YCbCr422Image};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<YCbCr422Image>,
    pub rgb_image: MainOutput<Image>,
    pub rgb_image_camera_info: MainOutput<CameraInfo>,
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
        let rgbd_sensors = context.hardware_interface.read_rgbd_sensors()?;
        let rgb_image = rgbd_sensors.rgb;
        let rgb_image_camera_info = rgbd_sensors.rgb_camera_info;
        let image: YCbCr422Image = (&(*rgb_image)).into();

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
            rgb_image: (*rgb_image).into(),
            rgb_image_camera_info: rgb_image_camera_info.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
