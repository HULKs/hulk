use std::sync::Arc;

use eframe::egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::backend::RobotBackend;

pub struct PanelCreationContext<'a> {
    pub backend: Arc<RobotBackend>,
    pub value: Option<&'a Value>,
    pub egui_context: Context,
}

pub struct PanelUiContext<'a> {
    pub backend: &'a Arc<RobotBackend>,
    pub egui_context: Context,
}

pub trait Panel {
    const STORAGE_ID: &'static str;
    const DISPLAY_NAME: &'static str;

    fn new(context: PanelCreationContext<'_>) -> Self;

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>);

    fn save(&self) -> Value {
        json!({})
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPanel {
    pub kind: String,
    #[serde(default)]
    pub state: Value,
}
