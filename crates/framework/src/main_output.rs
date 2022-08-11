#[derive(Default)]
pub struct MainOutput<DataType> {
    pub value: DataType,
}

impl<DataType> From<DataType> for MainOutput<DataType> {
    fn from(value: DataType) -> Self {
        Self { value }
    }
}
