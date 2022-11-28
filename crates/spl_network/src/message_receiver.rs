use std::{thread::sleep, time::Duration};

use color_eyre::Result;
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
        sleep(Duration::from_secs(1));
        self.value += *context.step;
        // context.hardware_interface.print_number(42);
        Ok(MainOutputs {
            value1: Some(self.value).into(),
        })
    }
}
