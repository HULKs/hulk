use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use serde::{Deserialize, Serialize};
use types::cycle_time::CycleTime;

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
    pub image_left_raw: MainOutput<Image>,
    pub image_left_raw_camera_info: MainOutput<CameraInfo>,
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
        let mut image_left_raw = context.hardware_interface.read_image_left_raw()?;
        let image_left_raw_camera_info = context
            .hardware_interface
            .read_image_left_raw_camera_info()?;

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        if (image_left_raw.height, image_left_raw.width) == (1088, 1280)
            && image_left_raw.encoding == "nv12"
        {
            image_left_raw.subsample_nv12_by_half_in_place()?;
        }

        let height = image_left_raw.height;
        let width = image_left_raw.width;
        assert_eq!((height, width), (544, 640));

        Ok(MainOutputs {
            image_left_raw: image_left_raw.into(),
            image_left_raw_camera_info: image_left_raw_camera_info.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
