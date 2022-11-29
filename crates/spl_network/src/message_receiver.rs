use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::MainOutput;
use types::hardware::Interface;

pub struct Counter {
    value: usize,
}

#[context]
pub struct NewContext {
    pub initial_value: Parameter<usize, "message_receiver/initial_value">,
}

#[context]
pub struct CycleContext {
    pub step: Parameter<usize, "message_receiver/step">,
    pub hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub value1: MainOutput<Option<usize>>,
}

impl Counter {
    pub fn new(context: NewContext) -> Result<Self> {
        Ok(Self {
            value: *context.initial_value,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let _message = context
            .hardware_interface
            .read_from_network()
            .wrap_err("failed to read from network")?;
        self.value += *context.step;
        Ok(MainOutputs {
            value1: Some(self.value).into(),
        })
    }
}
