use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, SensorData};

pub struct SensorDataReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

#[context]
pub struct MainOutputs {
    pub sensor_data: MainOutput<SensorData>,
}

impl SensorDataReceiver {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let sensor_data = context
            .hardware_interface
            .read_from_sensors()
            .wrap_err("failed to read from sensors")?;
        Ok(MainOutputs {
            sensor_data: sensor_data.into(),
        })
    }
}
