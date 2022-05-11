#[allow(dead_code)]
pub struct AdditionalOutput<'a, T> {
    is_subscribed: bool,
    data: &'a mut Option<T>,
}

impl<'a, T> AdditionalOutput<'a, T> {
    #[allow(dead_code)]
    pub fn new(is_subscribed: bool, data: &'a mut Option<T>) -> Self {
        Self {
            is_subscribed,
            data,
        }
    }

    #[allow(dead_code)]
    pub fn on_subscription<Callback>(&mut self, callback: Callback)
    where
        Callback: FnOnce() -> T,
    {
        if self.is_subscribed {
            *self.data = Some(callback())
        }
    }

    pub fn is_subscribed(&self) -> bool {
        self.is_subscribed
    }
}
