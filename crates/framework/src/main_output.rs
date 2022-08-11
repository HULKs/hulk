#[derive(Default)]
pub struct MainOutput<T> {
    pub value: T,
}

impl<T> From<T> for MainOutput<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}
