use std::{
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::TimeInterface;
use serde::{Deserialize, Serialize};
use types::cycle_time::CycleTime;

#[derive(Deserialize, Serialize)]
pub struct Timer {
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
    pub cycle_time: MainOutput<CycleTime>,
}

impl Timer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        sleep(Duration::from_millis(100));

        let now = context.hardware_interface.get_now();

        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };

        self.last_cycle_start = now;

        Ok(MainOutputs {
            cycle_time: cycle_time.into(),
        })
    }
}
