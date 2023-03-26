use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, ycbcr422_image::YCbCr422Image, CameraPosition, CycleTime};

use crate::CyclerInstance;

pub struct ImageReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
    pub instance: CyclerInstance,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<YCbCr422Image>,
    pub cycle_time: MainOutput<CycleTime>,
}

impl ImageReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(match context.instance {
                CyclerInstance::VisionTop => CameraPosition::Top,
                CyclerInstance::VisionBottom => CameraPosition::Bottom,
            })?;
        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("Nao time has run backwards"),
        };
        self.last_cycle_start = now;
        Ok(MainOutputs {
            image: image.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
