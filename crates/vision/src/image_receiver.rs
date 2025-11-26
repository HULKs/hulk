use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{RGBDSensorsInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::{ball_detection::BallPercept, cycle_time::CycleTime, ycbcr422_image::YCbCr422Image};

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
    pub image: MainOutput<YCbCr422Image>,
    pub cycle_time: MainOutput<CycleTime>,
    pub balls: MainOutput<Option<Vec<BallPercept>>>,
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
        let ycbcr422_image: YCbCr422Image = rgbd_image.rgb.as_ref().into();

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
            balls: None.into(),
        })
    }
}
