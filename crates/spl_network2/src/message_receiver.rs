use std::ops::Deref;

use context_attribute::context;

pub struct Parameter<'context, T> {
    value: &'context T,
}

impl<'context, T> Deref for Parameter<'context, T> {
    type Target = &'context T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

type MainOutput<T> = Option<T>;

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
pub struct MainOutputs {
    pub value: MainOutput<usize>,
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
            value: Some(self.value),
        })
    }
}
