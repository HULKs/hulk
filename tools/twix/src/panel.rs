use std::sync::Arc;

use serde_json::{Value, json};

use crate::backend::TwixBackend;

pub struct PanelCreationContext<'a> {
    pub backend: Arc<TwixBackend>,
    pub value: Option<&'a Value>,
}

pub trait Panel<'a> {
    const NAME: &'static str;
    fn new(context: PanelCreationContext<'a>) -> Self;
    fn save(&self) -> Value {
        json!({})
    }
}
