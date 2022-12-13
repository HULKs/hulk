use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, CycleInfo, SensorData};

pub struct SensorDataReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

#[context]
pub struct MainOutputs {
    pub sensor_data: MainOutput<SensorData>,
    pub cycle_info: MainOutput<CycleInfo>,
}

impl SensorDataReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let sensor_data = context
            .hardware_interface
            .read_from_sensors()
            .wrap_err("failed to read from sensors")?;
        let now = context.hardware_interface.get_now();
        let cycle_info = CycleInfo {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("Nao time has run backwards"),
        };
        self.last_cycle_start = now;
        Ok(MainOutputs {
            sensor_data: sensor_data.into(),
            cycle_info: cycle_info.into(),
        })
    }
}
