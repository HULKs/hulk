use std::sync::Arc;

use color_eyre::{Report, eyre::Context as _};
use eframe::egui::{TextEdit, Ui};
use hulk_widgets::CompletionEdit;
use ros_z::{dynamic::DynamicPayload, pubsub::PublicationId, time::Time};
use ros_z_debug::{DynamicTopicObservation, SampleRecord, TopicObservationStatus};
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
    status::format_topic_observation_status,
};

pub struct TextPanel {
    topic_editor: String,
    topic: String,
    pretty: bool,
    observation: ObservationState,
}

enum ObservationState {
    Idle,
    Observing(Box<ObservedTopic>),
    Error(String),
}

struct ObservedTopic {
    observation: DynamicTopicObservation,
    _repaint: ObservationRepaint,
    render_cache: RenderedRecordCache,
}

#[derive(Default)]
struct RenderedRecordCache {
    sample: Option<Arc<SampleRecord<DynamicPayload>>>,
    metadata: Option<RenderedMetadata>,
    value: Option<Value>,
    pretty: Option<String>,
    compact: Option<String>,
}

struct RenderedMetadata {
    resolved_topic: String,
    type_name: String,
    source_time: String,
    transport_time: String,
    publication_id: String,
}

impl Panel for TextPanel {
    const STORAGE_ID: &'static str = "text";
    const DISPLAY_NAME: &'static str = "Text";

    fn new(context: PanelCreationContext<'_>) -> Self {
        let topic = context
            .value
            .and_then(|value| value.get("topic"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let pretty = context
            .value
            .and_then(|value| value.get("pretty"))
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let mut panel = Self {
            topic_editor: topic.clone(),
            topic,
            pretty,
            observation: ObservationState::Idle,
        };
        panel.recreate_observation(&context);
        panel
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Topic");
                let completions = Vec::<String>::new();
                let response = ui.add(CompletionEdit::new(
                    ui.id().with("topic"),
                    &completions,
                    &mut self.topic_editor,
                ));
                if response.changed() {
                    self.commit_topic(&context);
                }
                ui.checkbox(&mut self.pretty, "Pretty");
            });

            if self.topic.is_empty() {
                ui.label("Enter a topic.");
                return;
            }

            match &mut self.observation {
                ObservationState::Idle => {
                    ui.label("No observation.");
                }
                ObservationState::Error(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ObservationState::Observing(observed) => {
                    Self::render_status(ui, observed.observation.status());
                    observed.render_cache.refresh(&observed.observation);

                    let Some(metadata) = observed.render_cache.metadata() else {
                        ui.label("Waiting for first sample.");
                        return;
                    };
                    Self::render_metadata(ui, metadata);
                    ui.separator();

                    if let Some(rendered) = observed.render_cache.rendered_json_buffer(self.pretty)
                    {
                        ui.add(
                            TextEdit::multiline(rendered)
                                .font(eframe::egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    }
                }
            }
        });
    }

    fn save(&self) -> Value {
        json!({
            "topic": self.topic,
            "pretty": self.pretty,
        })
    }
}

impl TextPanel {
    fn recreate_observation<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        self.observation = ObservationState::Idle;

        if self.topic.is_empty() {
            return;
        }

        match create_observation(context, &self.topic) {
            Ok((observation, repaint)) => {
                self.observation = ObservationState::Observing(Box::new(ObservedTopic {
                    observation,
                    _repaint: repaint,
                    render_cache: RenderedRecordCache::default(),
                }));
            }
            Err(error) => {
                self.observation = ObservationState::Error(format!("{error:#}"));
            }
        }
    }

    fn commit_topic<C>(&mut self, context: &C)
    where
        C: ObservationContext,
    {
        let next_topic = self.topic_editor.trim().to_string();
        if next_topic == self.topic {
            return;
        }
        self.topic = next_topic;
        self.recreate_observation(context);
    }

    fn render_metadata(ui: &mut Ui, metadata: &RenderedMetadata) {
        ui.horizontal_wrapped(|ui| {
            ui.label("topic:");
            ui.monospace(&metadata.resolved_topic);
            ui.separator();
            ui.label("type:");
            ui.monospace(&metadata.type_name);
            ui.separator();
            ui.label("source:");
            ui.monospace(&metadata.source_time);
            ui.separator();
            ui.label("transport:");
            ui.monospace(&metadata.transport_time);
            ui.separator();
            ui.label("publication:");
            ui.monospace(&metadata.publication_id);
        });
    }

    fn render_status(ui: &mut Ui, status: TopicObservationStatus) {
        ui.horizontal_wrapped(|ui| {
            ui.label("status:");
            ui.monospace(format_topic_observation_status(status));
        });
    }
}

impl RenderedRecordCache {
    fn refresh(&mut self, observation: &DynamicTopicObservation) {
        let sample = observation.latest();
        if same_sample(self.sample.as_ref(), sample.as_ref()) {
            return;
        }

        self.sample = sample;
        self.metadata = None;
        self.value = None;
        self.pretty = None;
        self.compact = None;

        if self.sample.is_none() {
            return;
        }

        if let Some(record) = observation.latest_json_record() {
            self.metadata = Some(RenderedMetadata::from(&record));
            self.value = Some(record.value);
        } else {
            self.sample = None;
        }
    }

    fn metadata(&self) -> Option<&RenderedMetadata> {
        self.metadata.as_ref()
    }

    fn rendered_json_buffer(&mut self, pretty: bool) -> Option<&mut String> {
        let value = self.value.as_ref()?;
        let rendered = if pretty {
            &mut self.pretty
        } else {
            &mut self.compact
        };

        if rendered.is_none() {
            *rendered = Some(render_json(value, pretty));
        }

        rendered.as_mut()
    }

    #[cfg(test)]
    fn replace_json_for_test(&mut self, value: Value) {
        self.value = Some(value);
        self.pretty = None;
        self.compact = None;
    }
}

impl From<&SampleRecord<Value>> for RenderedMetadata {
    fn from(record: &SampleRecord<Value>) -> Self {
        Self {
            resolved_topic: record.metadata.resolved_topic.clone(),
            type_name: record.metadata.type_info.name.to_string(),
            source_time: format_time(record.source_time),
            transport_time: record
                .transport_time
                .map(format_time)
                .unwrap_or_else(|| "none".to_string()),
            publication_id: format_publication_id(record.publication_id),
        }
    }
}

fn same_sample(
    current: Option<&Arc<SampleRecord<DynamicPayload>>>,
    next: Option<&Arc<SampleRecord<DynamicPayload>>>,
) -> bool {
    match (current, next) {
        (Some(current), Some(next)) => Arc::ptr_eq(current, next),
        (None, None) => true,
        _ => false,
    }
}

fn render_json(value: &Value, pretty: bool) -> String {
    let rendered = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    };

    rendered.unwrap_or_else(|error| format!("failed to render JSON: {error}"))
}

fn create_observation(
    context: &impl ObservationContext,
    topic: &str,
) -> Result<(DynamicTopicObservation, ObservationRepaint), Report> {
    let runtime_handle = context.backend().runtime_handle().clone();
    // ros_z_debug spawns observation tasks internally and needs a current runtime.
    let _runtime_context = runtime_handle.enter();
    let observation = context
        .backend()
        .observer()
        .observe_dynamic(topic)
        .wrap_err("failed to create dynamic topic observation")?
        .spawn();
    let repaint = observation.repaint_on_updates(context);
    Ok((observation, repaint))
}

fn format_time(time: Time) -> String {
    format!("{} ns", time.as_nanos())
}

fn format_publication_id(publication_id: PublicationId) -> String {
    format!("{publication_id:#}")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eframe::egui::Context;
    use ros_z::{EndpointGlobalId, pubsub::Received, time::Time};
    use serde_json::json;

    use crate::{backend::RobotBackend, panel::PanelCreationContext};

    use super::{ObservationState, Panel, RenderedRecordCache, TextPanel, format_publication_id};

    fn publication_id() -> ros_z::pubsub::PublicationId {
        Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 42,
            source_global_id: EndpointGlobalId::from([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ]),
        }
        .publication_id()
    }

    #[test]
    fn render_cache_reuses_serialized_json_for_unchanged_sample_and_format() {
        let mut cache = RenderedRecordCache::default();
        cache.replace_json_for_test(json!({ "answer": 42 }));

        let first = cache.rendered_json_buffer(true).unwrap().as_ptr();
        let second = cache.rendered_json_buffer(true).unwrap().as_ptr();

        assert_eq!(first, second);
    }

    #[test]
    fn render_cache_exposes_mutable_display_buffer_for_unchanged_sample_and_format() {
        let mut cache = RenderedRecordCache::default();
        cache.replace_json_for_test(json!({ "answer": 42 }));

        let rendered = cache.rendered_json_buffer(true).unwrap();
        rendered.push_str("\nlocal display state");

        assert!(
            cache
                .rendered_json_buffer(true)
                .unwrap()
                .ends_with("local display state")
        );
    }

    #[test]
    fn metadata_formats_compact_publication_id() {
        assert_eq!(
            format_publication_id(publication_id()),
            "01020304…0d0e0f10#42"
        );
    }

    #[test]
    fn save_preserves_topic_and_pretty_flag() {
        let panel = TextPanel {
            topic_editor: "/draft/topic".to_string(),
            topic: "/output/text".to_string(),
            pretty: false,
            observation: ObservationState::Idle,
        };

        assert_eq!(
            panel.save(),
            json!({
                "topic": "/output/text",
                "pretty": false,
            })
        );
    }

    #[test]
    fn new_restores_saved_topic_without_current_tokio_runtime() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let backend = Arc::new(
            runtime
                .block_on(RobotBackend::new(
                    runtime.handle().clone(),
                    None,
                    "/".to_string(),
                ))
                .expect("backend should build"),
        );
        let saved = json!({
            "topic": "/output/text",
            "pretty": true,
        });

        let panel = TextPanel::new(PanelCreationContext {
            backend,
            value: Some(&saved),
            egui_context: Context::default(),
        });

        assert_eq!(panel.topic, "/output/text");
    }
}
