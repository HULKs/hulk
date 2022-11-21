use std::time::SystemTime;

use context_attribute::context;
use framework::{AdditionalOutput, HistoricInput, MainOutput, PerceptionInput};
use hardware::HardwareInterface;

// TODO: dieses Modul weg, weil es nur zum Testen war?
pub struct MessageReceiver {
    value: usize,
}

#[context]
pub struct NewContext {
    pub hardware_interface: HardwareInterface,
    pub initial_value: Parameter<usize, "message_receiver/initial_value">,
    pub value: PersistentState<usize, "message_receiver/value">,
}

#[context]
pub struct CycleContext {
    pub step: Parameter<usize, "message_receiver/step">,
    pub test_a: Parameter<usize, "a/a/a">,
    pub test_b: Parameter<Option<usize>, "b?/a/a">,
    pub test_c: Parameter<Option<usize>, "c?/a?/a">,
    pub test_d: Parameter<Option<usize>, "d?/a?/a?">,
    pub test_e: Parameter<Option<usize>, "e/a?/a?">,
    pub test_f: Parameter<Option<usize>, "f/a/a?">,
    pub hardware_interface: HardwareInterface,
    pub test: PerceptionInput<Option<usize>, "SplNetwork", "value1?">,
    pub test2: HistoricInput<Option<usize>, "value?">,
    pub value: PersistentState<usize, "message_receiver/value">,
    pub output: AdditionalOutput<usize, "message_receiver/output">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub value: MainOutput<Option<usize>>,
}

impl MessageReceiver {
    pub fn new<Interface>(context: NewContext<Interface>) -> anyhow::Result<Self>
    where
        Interface: HardwareInterface,
    {
        context.hardware_interface.print_number(42);
        *context.value = 42;
        Ok(Self {
            value: *context.initial_value,
        })
    }

    pub fn cycle<Interface>(
        &mut self,
        mut context: CycleContext<Interface>,
    ) -> anyhow::Result<MainOutputs>
    where
        Interface: HardwareInterface,
    {
        self.value += *context.step;
        *context.value = 1337;
        context.hardware_interface.print_number(42);
        context.output.fill_on_subscription(|| 42);
        let _foo = context.test.persistent.is_empty();
        let _foo = context.test2.get(&SystemTime::now());
        // context.optional;
        Ok(MainOutputs {
            value: Some(self.value).into(),
        })
    }
}
