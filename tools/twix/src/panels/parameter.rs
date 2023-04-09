use std::sync::Arc;

use eframe::egui::{Response, ScrollArea, TextEdit, Ui, Widget};
use log::error;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct ParameterPanel {
    nao: Arc<Nao>,
    path: String,
    value_buffer: Option<ValueBuffer>,
    parameter_value: String,
    update_notify_sender: mpsc::Sender<()>,
    update_notify_receiver: mpsc::Receiver<()>,
}

impl Panel for ParameterPanel {
    const NAME: &'static str = "Parameter";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let path = match value.and_then(|value| value.get("subscribe_key")) {
            Some(Value::String(string)) => string.clone(),
            _ => String::new(),
        };
        let value_buffer = nao.subscribe_parameter(&path);
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        value_buffer.listen_to_updates(update_notify_sender.clone());

        Self {
            nao,
            path,
            value_buffer: Some(value_buffer),
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
                    let buffer = self.nao.subscribe_parameter(&self.path);
                    buffer.listen_to_updates(self.update_notify_sender.clone());
                    self.value_buffer = Some(buffer);
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
                        ui.label(format!("{error:#?}"));
                    }
                }
            }
        })
        .response
    }
}
