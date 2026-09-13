use std::sync::Arc;

use eframe::egui::{Context, Ui};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub fn saved_field<T: serde::de::DeserializeOwned>(
    value: &Value,
    key: &str,
) -> color_eyre::Result<Option<T>> {
    use color_eyre::eyre::WrapErr as _;
    value
        .get(key)
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .wrap_err_with(|| format!("invalid setting {key}"))
}

use crate::backend::RobotBackend;

pub struct PanelCreationContext<'a> {
    pub backend: Arc<RobotBackend>,
    pub value: Option<&'a Value>,
    pub egui_context: Context,
}

pub struct PanelUiContext<'a> {
    pub backend: &'a Arc<RobotBackend>,
    pub egui_context: &'a Context,
}

pub trait Panel {
    const STORAGE_ID: &'static str;
    const DISPLAY_NAME: &'static str;
    const ICON: &'static str;

    fn new(context: PanelCreationContext<'_>) -> Self;

    fn header_ui(&mut self, _ui: &mut Ui, _context: PanelUiContext<'_>) {}

    fn update(&mut self, _context: PanelUiContext<'_>) {}

    fn validate_state(value: &Value) -> color_eyre::Result<()>;

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
