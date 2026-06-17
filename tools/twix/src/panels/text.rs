use std::sync::Arc;

use chrono::{DateTime, Utc};
use eframe::egui::{Label, Response, ScrollArea, Sense, Ui, Widget};
use serde_json::{Value, json};

use crate::{
    backend::TwixBackend,
    panel::{Panel, PanelCreationContext},
    topic_completion_edit::TopicCompletionEdit,
    value_buffer::{BufferHandle, BufferHistory},
};

pub struct TextPanel {
    backend: Arc<TwixBackend>,
    topic: String,
    buffer: Option<BufferHandle<Value>>,
    show_all_topics: bool,
}

impl<'a> Panel<'a> for TextPanel {
    const NAME: &'static str = "Text";

    fn new(context: PanelCreationContext) -> Self {
        let topic = context
            .value
            .and_then(|value| value.get("topic").or_else(|| value.get("path")))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let buffer = if topic.is_empty() {
            None
        } else {
            Some(
                context
                    .backend
                    .subscribe_json(topic.clone(), BufferHistory::LatestOnly),
            )
        };
        Self {
            backend: context.backend,
            topic,
            buffer,
            show_all_topics: false,
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
        let edit_response = ui
            .horizontal(|ui| {
                ui.checkbox(&mut self.show_all_topics, "All topics");
                let catalog = self.backend.topic_catalog();
                let edit_response = if self.show_all_topics {
                    ui.add(TopicCompletionEdit::all_topics(
                        ui.id().with("text-panel"),
                        catalog,
                        &mut self.topic,
                    ))
                } else {
                    ui.add(TopicCompletionEdit::namespace_topics(
                        ui.id().with("text-panel"),
                        catalog,
                        &mut self.topic,
                    ))
                };
                if edit_response.changed() {
                    self.buffer = Some(
                        self.backend
                            .subscribe_json(self.topic.clone(), BufferHistory::LatestOnly),
                    );
                }
                if let Some(buffer) = &self.buffer
                    && let Ok(Some(timestamp)) = buffer.get_last_timestamp()
                {
                    let date: DateTime<Utc> = timestamp.into();
                    ui.label(date.format("%T%.3f").to_string());
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
