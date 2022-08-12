use context_attribute::context;
use framework::{HardwareInterface, MainOutput, Parameter, PerceptionInput};

pub struct MessageReceiver {
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
    pub test: PerceptionInput<usize, "SplNetwork", "value">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub value: MainOutput<Option<usize>>,
}

impl MessageReceiver {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            value: *context.initial_value,
        })
    }

    pub fn cycle<Interface>(
        &mut self,
        context: CycleContext<Interface>,
    ) -> anyhow::Result<MainOutputs>
    where
        Interface: hardware::HardwareInterface,
    {
        self.value += *context.step;
        context.hardware_interface.print_number(42);
        Ok(MainOutputs {
            value: Some(self.value).into(),
        })
    }
}
