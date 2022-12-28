#[derive(Debug)]
pub struct AdditionalOutput<'context, DataType> {
    is_subscribed: bool,
    data: &'context mut Option<DataType>,
}

impl<'context, DataType> AdditionalOutput<'context, DataType> {
    pub fn new(is_subscribed: bool, data: &'context mut Option<DataType>) -> Self {
        Self {
            is_subscribed,
            data,
        }
    }

    pub fn fill_if_subscribed<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce() -> DataType,
    {
        if self.is_subscribed {
            *self.data = Some(callback())
        }
    }

    pub fn mutate_if_subscribed<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce(&mut Option<DataType>),
    {
        if self.is_subscribed {
            callback(self.data);
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }
}
