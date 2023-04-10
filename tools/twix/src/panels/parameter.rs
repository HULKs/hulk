use std::sync::Arc;

use eframe::egui::{Response, ScrollArea, TextEdit, Ui, Widget};
use log::{error, info};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::{
    completion_edit::CompletionEdit, nao::Nao, panel::Panel,
    repository_configuration_handler::RepositoryConfigurationHandler, value_buffer::ValueBuffer,
};

pub struct ParameterPanel {
    nao: Arc<Nao>,
    path: String,
    current_url: Option<String>,
    repository_configuration_handler: RepositoryConfigurationHandler,
    value_buffer: Option<ValueBuffer>,
    parameter_value: String,
    update_notify_sender: mpsc::Sender<()>,
    update_notify_receiver: mpsc::Receiver<()>,
}

fn subscribe(
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

        let connection_url = nao.get_address();
        let repository_configuration_handler = RepositoryConfigurationHandler::new();
        repository_configuration_handler.print_nao_ids(connection_url.clone());

        Self {
            nao,
            path,
            value_buffer,
            current_url: connection_url,
            repository_configuration_handler,
            value_buffer: None,
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
        {
            let current_address = self.nao.get_address();
            if self.current_url != current_address {
                self.current_url = current_address;
                self.repository_configuration_handler
                    .print_nao_ids(self.current_url.clone());
            }
        }

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

                ui.add_enabled_ui(settable, |ui| {
                    if ui.button("Save to disk").clicked() {
                        match (
                            serde_json::from_str::<Value>(&self.parameter_value),
                            &self.current_url,
                            self.nao
                                .get_parameter_fields()
                                .map_or(false, |tree| tree.contains(&self.path)),
                        ) {
                            (Ok(value), Some(url), true) => {
                                if let Ok((hardware_ids, nao_id)) = self
                                    .repository_configuration_handler
                                    .get_hardware_ids_from_url(url.as_str())
                                {
                                    let status = self
                                        .repository_configuration_handler
                                        .merge_head_configuration_to_repository(
                                            hardware_ids.head_id.as_str(),
                                            &self.path,
                                            &value,
                                        );
                                    let message_part = format!(
                                        "configuration `{}` for Nao `{}` with head `{}`",
                                        self.path, nao_id, hardware_ids.head_id
                                    );
                                    match status {
                                        Ok(_) => {
                                            info!("Successfully wrote {}", message_part);
                                        }
                                        Err(error) => {
                                            error!("Failed to write {message_part} : {error:#?}")
                                        }
                                    }
                                } else {
                                    error!("Failed to locate Nao HW IDs. Cannot save to disk.")
                                }
                            }
                            (Err(error), _, _) => {
                                error!("Failed to serialize parameter value: {error:#?}")
                            }
                            (_, _, false) => {
                                error!(
                                    "Failed to save value to disk: path \"{}\" does not exist",
                                    self.path
                                )
                            }
                            (_, None, _) => {
                                error!("Invalid URL")
                            }
                        };
                    }
                });
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
