pub struct AdditionalOutput<'a, T> {
    is_subscribed: bool,
    data: &'a mut Option<T>,
}

impl<'a, T> AdditionalOutput<'a, T> {
    pub fn new(is_subscribed: bool, data: &'a mut Option<T>) -> Self {
        Self {
            is_subscribed,
            data,
        }
    }

    pub fn fill_on_subscription<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce() -> T,
    {
        if self.is_subscribed {
            *self.data = Some(callback())
        }
    }

    pub fn mutate_on_subscription<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce(&mut Option<T>),
    {
        if self.is_subscribed {
            callback(self.data);
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }
}
