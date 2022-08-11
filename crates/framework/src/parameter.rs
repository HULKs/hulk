use std::ops::Deref;

pub struct Parameter<'context, T> {
    value: &'context T,
}

impl<'context, T> Deref for Parameter<'context, T> {
    type Target = &'context T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
