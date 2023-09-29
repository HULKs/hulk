use std::time::{Duration, Instant};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{CameraInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image, camera_result::SequenceNumber};
use framework::{AdditionalOutput, MainOutput};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_sequence_number: SequenceNumber,
    last_cycle_start: Instant,
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
            last_sequence_number: Default::default(),
            last_cycle_start: Instant::now(),
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl CameraInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        context
            .cycle_time
            .fill_if_subscribed(|| self.last_cycle_start.elapsed());

        let camera_result = context
            .hardware_interface
            .read_from_camera(*context.camera_position, &self.last_sequence_number)?;
        self.last_sequence_number = camera_result.sequence_number.clone();
        self.last_cycle_start = Instant::now();
        Ok(MainOutputs {
            image: camera_result.image.clone().into(),
        })
    }
}
