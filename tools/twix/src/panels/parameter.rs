use std::sync::Arc;

use crate::{
    completion_edit::CompletionEdit, log_error::LogError, nao::Nao, panel::Panel,
    value_buffer::BufferHandle,
};
use color_eyre::{
    eyre::{eyre, Error},
    Result,
};
use communication::messages::TextOrBinary;
use eframe::egui::{Response, ScrollArea, TextEdit, Ui, Widget};
use log::error;
use parameters::directory::Scope;
use serde_json::{json, Value};

pub struct ParameterPanel {
    nao: Arc<Nao>,
    path: String,
    buffer: Option<BufferHandle<Value>>,
    parameter_value: Result<String>,
}

impl Panel for ParameterPanel {
    const NAME: &'static str = "Parameter";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let path = value
            .and_then(|value| value.get("path"))
            .and_then(|path| path.as_str());

        let value_buffer = path.map(|path| nao.subscribe_json(path));

        Self {
            nao,
            path: path.unwrap_or("").to_string(),
            buffer: value_buffer,
            parameter_value: Err(eyre!("no subscription yet")),
        }
    }
    fn save(&self) -> Value {
        json!({
            "path": self.path.clone()
        })
    }
}

impl Widget for &mut ParameterPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let path_edit =
                    CompletionEdit::writable_paths(&mut self.path, self.nao.as_ref()).ui(ui);
                if path_edit.changed() {
                    self.buffer = Some(self.nao.subscribe_json(&self.path));
                }
                let settable = self.buffer.is_some()
                    && self
                        .parameter_value
                        .as_ref()
                        .is_ok_and(|value| !value.is_empty());
                ui.add_enabled_ui(settable, |ui| {
                    if ui.button("Set").clicked() {
                        let serialized =
                            serde_json::from_str::<Value>(self.parameter_value.as_ref().unwrap());
                        match serialized {
                            Ok(value) => {
                                self.nao.write(self.path.clone(), TextOrBinary::Text(value));
                            }
                            Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                        }
                    }
                    if ui.button("Save to Head").clicked() {
                        let serialized =
                            serde_json::from_str::<Value>(self.parameter_value.as_ref().unwrap());
                        match serialized {
                            Ok(value) => {
                                self.nao
                                    .store_parameters(&self.path, value, Scope::current_head())
                                    .log_err();
                            }
                            Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                        }
                    }
                    if ui.button("Save to Body").clicked() {
                        let serialized =
                            serde_json::from_str::<Value>(self.parameter_value.as_ref().unwrap());
                        match serialized {
                            Ok(value) => {
                                self.nao
                                    .store_parameters(&self.path, value, Scope::current_body())
                                    .log_err();
                            }
                            Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                        }
                    }
                });
            });

            if let Some(buffer) = &mut self.buffer {
                if buffer.has_changed() {
                    buffer.mark_as_seen();
                    match buffer.get_last_value() {
                        Ok(Some(value)) => {
                            self.parameter_value =
                                serde_json::to_string_pretty(&value).map_err(Error::from);
                        }
                        Ok(None) => {
                            self.parameter_value = Err(eyre!("no data yet"));
                        }
                        Err(error) => {
                            self.parameter_value = Err(error);
                        }
                    }
                }
                match &mut self.parameter_value {
                    Ok(value) => {
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(value)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                    }
                    Err(error) => {
                        ui.label(format!("{error:#?}"));
                    }
                }
            }
        })
        .response
    }
}
