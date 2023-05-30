use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, CycleTime, Joints, SensorData};

pub struct SensorDataReceiver {
    last_cycle_start: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
    pub joint_calibration_offsets: Input<Joints<f32>, "joint_calibration_offsets">,
}

#[context]
pub struct MainOutputs {
    pub sensor_data: MainOutput<SensorData>,
    pub cycle_time: MainOutput<CycleTime>,
}

impl SensorDataReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_cycle_start: UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let mut sensor_data = context
            .hardware_interface
            .read_from_sensors()
            .wrap_err("failed to read from sensors")?;

        sensor_data.positions = sensor_data.positions - (*context.joint_calibration_offsets);

        let now = context.hardware_interface.get_now();
        let cycle_time = CycleTime {
            start_time: now,
            last_cycle_duration: now
                .duration_since(self.last_cycle_start)
                .expect("Nao time has run backwards"),
        };
        self.last_cycle_start = now;
        Ok(MainOutputs {
            sensor_data: sensor_data.into(),
            cycle_time: cycle_time.into(),
        })
    }
}
