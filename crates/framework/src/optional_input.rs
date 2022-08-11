use std::ops::Deref;

pub struct OptionalInput<'context, DataType> {
    value: &'context Option<DataType>,
}

impl<'context, DataType> Deref for OptionalInput<'context, DataType> {
    type Target = &'context Option<DataType>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
