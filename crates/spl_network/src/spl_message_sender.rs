use color_eyre::Result;
use context_attribute::context;
use types::hardware::Interface;

pub struct SplMessageSender {
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
    // pub optional: Input<Option<usize>, "Control", "value?">,
    // pub required: RequiredInput<Option<usize>, "Control", "value?">,
    pub required2: RequiredInput<Option<usize>, "value1?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl SplMessageSender {
    pub fn new(context: NewContext) -> Result<Self> {
        Ok(Self {
            value: *context.initial_value,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        self.value += *context.step;
        // context.hardware_interface.print_number(42);
        Ok(MainOutputs {})
    }
}
