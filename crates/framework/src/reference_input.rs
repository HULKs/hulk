use std::ops::{Deref, DerefMut};

pub struct ReferenceInput<'context, DataType> {
    value: &'context DataType,
}

impl<'context, DataType> Deref for ReferenceInput<'context, DataType> {
    type Target = DataType;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct MutableReferenceInput<'context, DataType> {
    value: &'context mut DataType,
}

impl<'context, DataType> Deref for MutableReferenceInput<'context, DataType> {
    type Target = DataType;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'context, DataType> DerefMut for MutableReferenceInput<'context, DataType> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}
