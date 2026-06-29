use std::sync::Arc;

use eframe::{egui::Context, egui_wgpu::RenderState};
use serde_json::{Value, json};

use crate::robot::Robot;

pub struct PanelCreationContext<'a> {
    pub robot: Arc<Robot>,
    pub value: Option<&'a Value>,
    pub wgpu_state: RenderState,
    pub egui_context: Context,
}

pub trait Panel<'a> {
    const NAME: &'static str;
    fn new(context: PanelCreationContext<'a>) -> Self;
    fn save(&self) -> Value {
        json!({})
    }
}
