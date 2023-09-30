use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
    camera_position: Parameter<CameraPosition, "image_receiver.$cycler_instance.camera_position">,
    cycle_time: AdditionalOutput<Duration, "cycle_time">,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<YCbCr422Image>,
}

impl ImageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl CameraInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        context
            .cycle_time
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
