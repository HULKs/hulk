use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::CameraInterface;
use serde::{Deserialize, Serialize};
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image, camera_result::SequenceNumber};

#[derive(Deserialize, Serialize)]
pub struct ImageReceiver {
    last_sequence_number: SequenceNumber,
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
}

impl ImageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_sequence_number: Default::default(),
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl CameraInterface>) -> Result<MainOutputs> {
        let camera_result = context
            .hardware_interface
            .read_from_camera(*context.camera_position, &self.last_sequence_number)?;
        self.last_sequence_number = camera_result.sequence_number.clone();

        Ok(MainOutputs {
            image: camera_result.image.clone().into(),
        })
    }
}
