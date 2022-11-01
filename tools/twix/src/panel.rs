use std::sync::Arc;

use serde_json::{json, Value};

use crate::nao::Nao;

pub trait Panel {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self;
    fn save(&self) -> Value {
        json!({})
    }
}
