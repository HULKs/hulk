use std::sync::Arc;

use chrono::{DateTime, Utc};
use eframe::egui::{Label, Response, ScrollArea, Sense, Ui, Widget};
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    topic_completion_edit::TopicCompletionEdit,
    value_buffer::BufferHandle,
};

pub struct TextPanel {
    robot: Arc<Robot>,
    topic: String,
    buffer: Option<BufferHandle<Value>>,
}

impl<'a> Panel<'a> for TextPanel {
    const NAME: &'static str = "Text";

    fn new(context: PanelCreationContext) -> Self {
        let topic = match context.value.and_then(|value| value.get("topic")) {
            Some(Value::String(string)) => string.to_string(),
            _ => String::new(),
        };
        let buffer = if !topic.is_empty() {
            Some(context.robot.subscribe_json(topic.clone()))
        } else {
            None
        };
        Self {
            robot: context.robot,
            topic,
            buffer,
        }
    }

    fn save(&self) -> Value {
        json!({
            "topic": self.topic.clone()
        })
    }
}

impl Widget for &mut TextPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let topic_state = self.robot.topic_list_state();
        let edit_response = ui
            .horizontal(|ui| {
                let edit_response = ui.add(TopicCompletionEdit::new(
                    ui.id().with("text-panel"),
                    &topic_state,
                    &mut self.topic,
                ));
                if edit_response.changed() {
                    self.buffer = Some(self.robot.subscribe_json(self.topic.clone()));
                }
                if let Some(buffer) = &self.buffer
                    && let Ok(Some(datum)) = buffer.get_last()
                {
                    let date: DateTime<Utc> = datum.timestamp.as_system_time().into();
                    ui.label(date.format("%T%.3f").to_string());
                    if let Some(source_timestamp) = datum.source_timestamp {
                        ui.label(format!("src {}", source_timestamp));
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
