use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use hardware::{CameraInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image};

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
    last_cycle_time: AdditionalOutput<Duration, "cycle_time">,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<YCbCr422Image>,
}

impl ImageReceiver {
    pub fn new(context: CreationContext<impl TimeInterface>) -> Result<Self> {
        Ok(Self {
            last_cycle_start: context.hardware_interface.get_now(),
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl CameraInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        context
            .last_cycle_time
            .fill_if_subscribed(|| self.last_cycle_start.elapsed().expect("time ran backwards"));

        let image = context
            .hardware_interface
            .read_from_camera(*context.camera_position)?;
        self.last_cycle_start = context.hardware_interface.get_now();

        Ok(MainOutputs {
            image: image.into(),
        })
    }
}
