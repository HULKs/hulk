use std::{str::FromStr, sync::Arc};

use communication::CyclerOutput;
use eframe::{
    egui::{ScrollArea, Widget},
    Storage,
};
use log::error;
use serde_json::{json, Value};

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct TextPanel {
    nao: Arc<Nao>,
    output: String,
    values: Option<ValueBuffer>,
}

impl Panel for TextPanel {
    const NAME: &'static str = "Text";

    fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>) -> Self {
        let output = match storage.and_then(|storage| storage.get_string("text_panel_output")) {
            Some(stored_output) => stored_output,
            None => String::new(),
        };
        let values = if !output.is_empty() {
            let output = CyclerOutput::from_str(&output);
            match output {
                Ok(output) => Some(nao.subscribe_output(output)),
                Err(error) => {
                    error!("Failed to subscribe: {error:?}");
                    None
                }
            }
        } else {
            None
        };
        Self {
            nao,
            output,
            values,
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        storage.set_string("text_panel_output", self.output.clone());
    }
}
impl TextPanel {
    pub fn new2(nao: Arc<Nao>, value: &Value) -> Self {
        let output = match value.get("subscribe_key") {
            Some(Value::String(string)) => string.to_string(),
            _ => String::new(),
        };
        let values = if !output.is_empty() {
            let output = CyclerOutput::from_str(&output);
            match output {
                Ok(output) => Some(nao.subscribe_output(output)),
                Err(error) => {
                    error!("Failed to subscribe: {error:?}");
                    None
                }
            }
        } else {
            None
        };
        Self {
            nao,
            output,
            values,
        }
    }

    pub fn save2(&self) -> Value {
        json!({
            "subscribe_key": self.output.clone()
        })
    }
}

impl Widget for &mut TextPanel {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let edit_response = ui.add(CompletionEdit::outputs(&mut self.output, self.nao.as_ref()));
        if edit_response.changed() {
            match CyclerOutput::from_str(&self.output) {
                Ok(output) => {
                    self.values = Some(self.nao.subscribe_output(output));
                }
                Err(error) => {
                    error!("Failed to subscribe: {error:#?}");
                }
            }
        }
        let scroll_area = ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.values
                    .as_ref()
                    .map(|buffer| match buffer.get_latest() {
                        Ok(value) => {
                            let content = match serde_json::to_string_pretty(&value) {
                                Ok(pretty_string) => pretty_string,
                                Err(error) => format!("{error:#?}"),
                            };
                            ui.label(content)
                        }
                        Err(error) => ui.label(format!("{error:#?}")),
                    })
            });
        if let Some(response) = scroll_area.inner {
            edit_response | response
        } else {
            edit_response
        }
    }
}
