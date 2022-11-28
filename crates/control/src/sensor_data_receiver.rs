use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, Ear, Joints, Leds, Rgb, SensorData};

pub struct SensorDataReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sensor_data: MainOutput<SensorData>,
}

impl SensorDataReceiver {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let sensor_data = context.hardware_interface.read_from_sensors()?;
        Ok(MainOutputs::default())
    }
}
