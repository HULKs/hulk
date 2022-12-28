use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{hardware::Interface, messages::IncomingMessage};

// TODO: dieses Modul weg, weil es nur zum Testen war?
pub struct MessageFilter {
    value: usize,
}

#[context]
pub struct CreationContext {
    pub hardware_interface: HardwareInterface,
    pub initial_value: Parameter<usize, "message_receiver.initial_value">,
    pub value: PersistentState<usize, "message_receiver.value">,
}

#[context]
pub struct CycleContext {
    pub step: Parameter<usize, "message_receiver.step">,
    // pub test_a: Parameter<usize, "a.a.a">,
    // pub test_b: Parameter<Option<usize>, "b?.a.a">,
    // pub test_c: Parameter<Option<usize>, "c?.a?.a">,
    // pub test_d: Parameter<Option<usize>, "d?.a?.a?">,
    // pub test_e: Parameter<Option<usize>, "e.a?.a?">,
    // pub test_f: Parameter<Option<usize>, "f.a.a?">,
    // pub hardware_interface: HardwareInterface,
    pub test: PerceptionInput<IncomingMessage, "SplNetwork", "message">,
    // // pub test2: HistoricInput<Option<usize>, "value?">,
    pub value: PersistentState<usize, "message_receiver.value">,
    // pub output: AdditionalOutput<usize, "message_receiver.output">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub value: MainOutput<Option<usize>>,
}

impl MessageFilter {
    pub fn new(context: CreationContext<impl Interface>) -> Result<Self> {
        // context.hardware_interface.print_number(42);
        *context.value = 42;
        Ok(Self {
            value: *context.initial_value,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.value += *context.step;
        *context.value = 1337;
        // context.hardware_interface.print_number(42);
        // context.output.fill_on_subscription(|| 42);
        // let _foo = context.test.persistent.is_empty();
        // let _foo = context.test2.get(&SystemTime::now());
        // context.optional;
        Ok(MainOutputs {
            value: Some(self.value).into(),
        })
    }
}
