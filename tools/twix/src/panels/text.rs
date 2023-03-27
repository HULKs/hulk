use std::{str::FromStr, sync::Arc};

use communication::client::CyclerOutput;
use eframe::egui::{Label, ScrollArea, Sense, Widget};
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

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let output = match value.and_then(|value| value.get("subscribe_key")) {
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

    fn save(&self) -> Value {
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
                                Err(error) => error.to_string(),
                            };
                            let label = ui.add(Label::new(&content).sense(Sense::click()));
                            if label.clicked() {
                                ui.output_mut(|output| output.copied_text = content);
                            }
                            label.on_hover_ui_at_pointer(|ui| {
                                ui.label("Click to copy");
                            })
                        }
                        Err(error) => ui.label(error),
                    })
            });
        if let Some(response) = scroll_area.inner {
            edit_response | response
        } else {
            edit_response
        }
    }
}
