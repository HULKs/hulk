use context_attribute::context;
use framework::MainOutput;
use hardware::HardwareInterface;

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
        Interface: HardwareInterface,
    {
        self.value += *context.step;
        // context.hardware_interface.print_number(42);
        Ok(MainOutputs {
            value1: Some(self.value).into(),
        })
    }
}
