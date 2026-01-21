use std::time::SystemTime;

use booster::FallDownState;
use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{FallDownStateInterface, TimeInterface};
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
    pub fall_down_state: MainOutput<FallDownState>,
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
        context: CycleContext<impl FallDownStateInterface + TimeInterface>,
    ) -> Result<MainOutputs> {
        let fall_down_state = context.hardware_interface.read_fall_down_state()?;

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("time ran backwards"),
        };
        self.last_cycle_start = now;

        Ok(MainOutputs {
            fall_down_state: fall_down_state.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
