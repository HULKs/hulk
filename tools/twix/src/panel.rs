use std::sync::Arc;

use eframe::egui_wgpu::RenderState;
use serde_json::{json, Value};

use crate::nao::Nao;

pub struct PanelCreationContext<'a> {
    pub nao: Arc<Nao>,
    pub value: Option<&'a Value>,
    pub wgpu_state: RenderState,
}

pub trait Panel<'a> {
    const NAME: &'static str;
    fn new(context: PanelCreationContext<'a>) -> Self;
    fn save(&self) -> Value {
        json!({})
    }
}
