use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, image::Image, CameraPosition};

use crate::CyclerInstance;

pub struct ImageReceiver {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
    pub instance: CyclerInstance,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<Image>,
}

impl ImageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(match context.instance {
                CyclerInstance::VisionTop => CameraPosition::Top,
                CyclerInstance::VisionBottom => CameraPosition::Bottom,
            })?;
        Ok(MainOutputs {
            image: image.into(),
        })
    }
}
