use context_attribute::context;
use framework::{MainOutput, Parameter};

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
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub value: MainOutput<Option<usize>>,
}

impl Counter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            value: **context.initial_value,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        self.value += **context.step;
        Ok(MainOutputs {
            value: Some(self.value).into(),
        })
    }
}
