use std::sync::Arc;

use crate::{
    completion_edit::CompletionEdit, nao::Nao, panel::Panel,
    repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};
use color_eyre::{eyre::Context, Result};
use eframe::egui::{Response, ScrollArea, TextEdit, Ui, Widget};
use log::error;
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct ParameterPanel {
    nao: Arc<Nao>,
    path: String,
    repository_parameters: Option<RepositoryParameters>,
    value_buffer: Option<ValueBuffer>,
    parameter_value: String,
    update_notify_sender: mpsc::Sender<()>,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub fn subscribe(
    nao: Arc<Nao>,
    path: &str,
    update_notify_sender: mpsc::Sender<()>,
) -> Option<ValueBuffer> {
    if path.is_empty() {
        return None;
    }

    let value_buffer = nao.subscribe_parameter(path);
    value_buffer.listen_to_updates(update_notify_sender);
    Some(value_buffer)
}

impl Panel for ParameterPanel {
    const NAME: &'static str = "Parameter";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let path = match value.and_then(|value| value.get("subscribe_key")) {
            Some(Value::String(string)) => string.clone(),
            _ => String::new(),
        };

        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        let value_buffer = subscribe(nao.clone(), &path, update_notify_sender.clone());

        Self {
            nao,
            path,
            repository_parameters: RepositoryParameters::try_default().ok(),
            value_buffer,
            parameter_value: String::new(),
            update_notify_sender,
            update_notify_receiver,
        }
    }
    fn save(&self) -> Value {
        json!({
            "subscribe_key": self.path.clone()
        })
    }
}

impl Widget for &mut ParameterPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let path_edit =
                    CompletionEdit::parameters(&mut self.path, self.nao.as_ref()).ui(ui);
                if path_edit.changed() {
                    self.value_buffer = subscribe(
                        self.nao.clone(),
                        &self.path,
                        self.update_notify_sender.clone(),
                    )
                }
                let settable = self.value_buffer.is_some() && !self.parameter_value.is_empty();
                ui.add_enabled_ui(settable, |ui| {
                    if ui.button("Set").clicked() {
                        match serde_json::from_str(&self.parameter_value) {
                            Ok(value) => {
                                self.nao.update_parameter_value(&self.path, value);
                            }
                            Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                        }
                    }
                });

                add_save_button(
                    ui,
                    &self.path,
                    || -> Result<Value> {
                        serde_json::from_str::<Value>(self.parameter_value.as_str()).wrap_err(
                            "Serialising the parameter string to serde_json::Value failed",
                        )
                    },
                    self.nao.clone(),
                    &self.repository_parameters,
                    settable,
                );
            });

            if let Some(buffer) = &self.value_buffer {
                match buffer.get_latest() {
                    Ok(value) => {
                        if self.update_notify_receiver.try_recv().is_ok() {
                            self.parameter_value = serde_json::to_string_pretty(&value).unwrap();
                        }
                        ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                TextEdit::multiline(&mut self.parameter_value)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                    }
                    Err(error) => {
                        ui.label(error);
                    }
                }
            }
        })
        .response
    }
}

pub fn add_save_button<SerdesJsonValueProvider>(
    ui: &mut Ui,
    parameter_path: &str,
    parameter_value_provider_fn: SerdesJsonValueProvider,
    nao: Arc<Nao>,
    repository_parameters: &Option<RepositoryParameters>,
    settable: bool,
) where
    SerdesJsonValueProvider: Fn() -> Result<serde_json::Value>,
{
    ui.add_enabled_ui(settable, |ui| {
        if ui.button("Save to disk").clicked() {
            if let Some(address) = nao.get_address() {
                match (parameter_value_provider_fn(), repository_parameters) {
                    (Ok(value), Some(repository_parameters)) => {
                        repository_parameters.write(&address, parameter_path.to_owned(), value);
                    }
                    (Err(error), _) => {
                        error!("Failed to serialize parameter value: {error:#?}")
                    }
                    _ => {
                        error!("Repository is not available, cannot save.")
                    }
                };
            }
        }
    });
}
