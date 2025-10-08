use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RGBDSensorsInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{
    camera_position::CameraPosition, cycle_time::CycleTime, ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<YCbCr422Image>,
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
        context: CycleContext<impl RGBDSensorsInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let rgbd_image = context.hardware_interface.read_rgbd_sensors()?;
        let ycbcr422_image: YCbCr422Image = (&rgbd_image.rgb).into();

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        Ok(MainOutputs {
            image: ycbcr422_image.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
