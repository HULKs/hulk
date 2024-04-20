use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{
    camera_position::CameraPosition, cycle_time::CycleTime, ycbcr422_image::YCbCr422Image,
};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

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
    pub fn new(context: CreationContext<impl TimeInterface>) -> Result<Self> {
        Ok(Self {
            last_cycle_start: context.hardware_interface.get_now(),
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl CameraInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(*context.camera_position)?;

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
            cycle_time: cycle_time.into(),
        })
    }
}
