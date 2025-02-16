use std::sync::Arc;

use chrono::{DateTime, Utc};
use eframe::egui::{Label, Response, ScrollArea, Sense, Ui, Widget};
use hulk_widgets::{NaoPathCompletionEdit, PathFilter};
use serde_json::{json, Value};

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct TextPanel {
    nao: Arc<Nao>,
    path: String,
    buffer: Option<BufferHandle<Value>>,
}

impl Panel for TextPanel {
    const NAME: &'static str = "Text";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let path = match value.and_then(|value| value.get("path")) {
            Some(Value::String(string)) => string.to_string(),
            _ => String::new(),
        };
        let buffer = if !path.is_empty() {
            Some(nao.subscribe_json(path.clone()))
        } else {
            None
        };
        Self { nao, path, buffer }
    }

    fn save(&self) -> Value {
        json!({
            "path": self.path.clone()
        })
    }
}

impl Widget for &mut TextPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let edit_response = ui
            .horizontal(|ui| {
                let edit_response = ui.add(NaoPathCompletionEdit::new(
                    ui.id().with("text-panel"),
                    self.nao.latest_paths(),
                    &mut self.path,
                    PathFilter::Readable,
                ));
                if edit_response.changed() {
                    self.buffer = Some(self.nao.subscribe_json(self.path.clone()));
                }
                if let Some(buffer) = &self.buffer {
                    if let Ok(Some(timestamp)) = buffer.get_last_timestamp() {
                        let date: DateTime<Utc> = timestamp.into();
                        ui.label(date.format("%T%.3f").to_string());
                    }
                }
                edit_response
            })
            .inner;
        let scroll_area = ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.buffer.as_ref().map(|buffer| match buffer.get_last() {
                    Ok(Some(datum)) => {
                        let content = match serde_json::to_string_pretty(&datum.value) {
                            Ok(pretty_string) => pretty_string,
                            Err(error) => error.to_string(),
                        };
                        let label = ui.add(Label::new(&content).sense(Sense::click()));
                        if label.clicked() {
                            ui.ctx().copy_text(content);
                        }
                        label.on_hover_ui_at_pointer(|ui| {
                            ui.label("Click to copy");
                        })
                    }
                    Err(error) => ui.label(error.to_string()),
                    Ok(None) => ui.label("no data available"),
                })
            });
        if let Some(response) = scroll_area.inner {
            edit_response | response
        } else {
            edit_response
        }
    }
}
