use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::CameraInterface;
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image};

pub struct ImageReceiver {}

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
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl CameraInterface>) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(*context.camera_position)?;
        Ok(MainOutputs {
            image: image.into(),
        })
    }
}
