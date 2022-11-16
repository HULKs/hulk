use std::ops::Deref;

pub struct Input<'context, DataType> {
    value: &'context Option<DataType>,
}

impl<'context, DataType> From<&'context Option<DataType>> for Input<'context, DataType> {
    fn from(value: &'context Option<DataType>) -> Self {
        Self { value }
    }
}

impl<'context, DataType> Deref for Input<'context, DataType> {
    type Target = &'context Option<DataType>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

// TODO: support other cyclers
