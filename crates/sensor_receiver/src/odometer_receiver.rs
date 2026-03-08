use std::time::SystemTime;

use booster::Odometer;
use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{OdometerInterface, TimeInterface};
use serde::{Deserialize, Serialize};
use types::cycle_time::CycleTime;

#[derive(Deserialize, Serialize)]
pub struct FallDownStateReceiver {
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
    pub odometer: MainOutput<Option<Odometer>>,
    pub cycle_time: MainOutput<CycleTime>,
}

impl FallDownStateReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl OdometerInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let odometer = Some(context.hardware_interface.get_odometer()?);
        println!("got odometer");

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        Ok(MainOutputs {
            odometer: odometer.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
