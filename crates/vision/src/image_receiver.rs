use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use ros2::sensor_msgs::image::Image;
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
    pub image: MainOutput<Image>,
    pub ycbcr422_image: MainOutput<YCbCr422Image>,
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
        let image = context.hardware_interface.read_image_left_raw()?;

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        let ycbcr422_image = YCbCr422Image::try_from(&image)?;

        Ok(MainOutputs {
            ycbcr422_image: ycbcr422_image.into(),
            image: image.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
