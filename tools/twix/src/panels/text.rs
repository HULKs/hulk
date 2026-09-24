use std::sync::Arc;

use color_eyre::{Report, eyre::Context as _};
use eframe::egui::{ScrollArea, TextEdit, Ui};
use ros_z::{
    dynamic::{DynamicPayload, SelectionError, ValuePath},
    pubsub::PublicationId,
    time::Time,
};
use ros_z_debug::{DynamicTopicObservation, SampleRecord, TopicObservationStatus};
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates},
    status::format_topic_observation_status,
    topic_source::TopicSourceEditor,
};

pub struct TextPanel {
    source: TopicSourceEditor,
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
    field_path: String,
    selection_error: Option<SelectionError>,
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
    const ICON: &'static str = egui_material_icons::icons::ICON_TEXT_FIELDS.codepoint;

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
        let field_path = context
            .value
            .and_then(|value| value.get("field_path"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        let mut panel = Self {
            source: TopicSourceEditor::new(topic, field_path),
            pretty,
            observation: ObservationState::Idle,
        };
        panel.recreate_observation(&context);
        panel
    }

    fn header_ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        let sample = match &self.observation {
            ObservationState::Observing(observed) => observed.observation.latest(),
            _ => None,
        };
        ui.vertical(|ui| {
            ui.spacing_mut().text_edit_width = ui
                .spacing()
                .text_edit_width
                .min((ui.available_width() - 55.0).max(0.0));
            ui.horizontal_wrapped(|ui| {
                if self.source.ui(
                    ui,
                    context.backend,
                    sample.as_ref().map(|sample| &sample.value),
                ) {
                    self.recreate_observation(&context);
                }
                ui.checkbox(&mut self.pretty, "Pretty");
            });
        });
    }

    fn ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        if self.source.topic().is_empty() {
            ui.label("Enter a topic.");
            return;
        }

        ScrollArea::both()
            .id_salt("text-panel-content")
            .auto_shrink([false; 2])
            .show(ui, |ui| match &mut self.observation {
                ObservationState::Idle => {
                    ui.label("No observation.");
                }
                ObservationState::Error(error) => {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                ObservationState::Observing(observed) => {
                    Self::render_status(ui, observed.observation.status());
                    observed
                        .render_cache
                        .refresh(observed.observation.latest(), self.source.field_path());

                    let Some(metadata) = observed.render_cache.metadata() else {
                        ui.label("Waiting for first sample.");
                        return;
                    };
                    Self::render_metadata(ui, metadata);
                    ui.separator();

                    if let Some(error) = &observed.render_cache.selection_error {
                        let color = if error.is_unavailable() {
                            ui.visuals().warn_fg_color
                        } else {
                            ui.visuals().error_fg_color
                        };
                        ui.colored_label(color, error.to_string());
                    }

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
            });
    }

    fn save(&self) -> Value {
        json!({
            "topic": self.source.topic(),
            "field_path": self.source.field_path(),
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

        if self.source.topic().is_empty() {
            return;
        }

        match create_observation(context, self.source.topic()) {
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
    fn refresh(&mut self, sample: Option<Arc<SampleRecord<DynamicPayload>>>, field_path: &str) {
        if same_sample(self.sample.as_ref(), sample.as_ref()) && self.field_path == field_path {
            return;
        }

        self.sample = sample;
        self.field_path = field_path.to_owned();
        self.selection_error = None;
        self.metadata = None;
        self.value = None;
        self.pretty = None;
        self.compact = None;

        let Some(record) = &self.sample else {
            return;
        };
        self.metadata = Some(RenderedMetadata::from(record.as_ref()));
        match field_path
            .parse::<ValuePath>()
            .and_then(|path| path.select(&record.value))
        {
            Ok(value) => self.value = Some(value.to_json(Default::default())),
            Err(error) => self.selection_error = Some(error),
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

impl From<&SampleRecord<DynamicPayload>> for RenderedMetadata {
    fn from(record: &SampleRecord<DynamicPayload>) -> Self {
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
    fn save_preserves_topic_field_and_pretty_flag() {
        let panel = TextPanel {
            source: crate::topic_source::TopicSourceEditor::new(
                "/output/text".to_owned(),
                "pose.x".to_owned(),
            ),
            pretty: false,
            observation: ObservationState::Idle,
        };

        assert_eq!(
            panel.save(),
            json!({
                "topic": "/output/text",
                "field_path": "pose.x",
                "pretty": false,
            })
        );
        assert_eq!(
            serde_json::to_value(crate::SelectablePanel::TextPanel(Box::new(panel))).unwrap(),
            json!({"kind": "text", "state": {"topic": "/output/text", "field_path": "pose.x", "pretty": false}})
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
        for field_path in [None, Some("pose.x")] {
            let mut saved = json!({
                "topic": "/output/text",
                "pretty": true,
            });
            if let Some(path) = field_path {
                saved["field_path"] = path.into();
            }
            let panel = TextPanel::new(PanelCreationContext {
                backend: Arc::clone(&backend),
                value: Some(&saved),
                egui_context: Context::default(),
            });
            assert_eq!(panel.source.topic(), "/output/text");
            assert_eq!(panel.source.field_path(), field_path.unwrap_or_default());
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn source_controls_fit_narrow_panel_headers() {
        use crate::panel::PanelUiContext;
        use eframe::egui::{CentralPanel, RawInput, Rect, vec2};
        let backend = Arc::new(
            RobotBackend::new(tokio::runtime::Handle::current(), None, "/".into())
                .await
                .unwrap(),
        );
        for width in [240.0, 800.0] {
            let context = Context::default();
            let mut panel = TextPanel {
                source: crate::topic_source::TopicSourceEditor::new(
                    "/topic".into(),
                    "pose.x".into(),
                ),
                pretty: true,
                observation: ObservationState::Idle,
            };
            let _ = context.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(Default::default(), vec2(width, 600.0))),
                    ..Default::default()
                },
                |ui| {
                    CentralPanel::default().show(ui, |ui| {
                        let available = ui.available_width();
                        let response = ui.horizontal(|ui| {
                            panel.header_ui(
                                ui,
                                PanelUiContext {
                                    backend: &backend,
                                    egui_context: &context,
                                },
                            )
                        });
                        assert!(
                            response.response.rect.width() <= available + 1.0,
                            "width={width}, actual={:?}",
                            response.response.rect
                        );
                    });
                },
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn changing_selection_reuses_sample_and_metadata_and_clears_stale_rendering() {
        use ros_z::context::ContextBuilder;
        use ros_z_debug::{TopicObserver, TopicObserverOptions};
        use std::time::Duration;

        let context = ContextBuilder::default()
            .disable_multicast_scouting()
            .with_json("connect/endpoints", json!([]))
            .build()
            .await
            .unwrap();
        let node = Arc::new(
            context
                .create_node("twix_selection_test")
                .build()
                .await
                .unwrap(),
        );
        let publisher = node
            .publisher::<Vec<f64>>("/twix_selection_test")
            .build()
            .await
            .unwrap();
        let observer = TopicObserver::new(node, TopicObserverOptions::with_namespace("/").unwrap());
        let observation = observer
            .observe_dynamic("/twix_selection_test")
            .unwrap()
            .spawn();
        let sample = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                publisher.publish(&vec![7.0, 9.0]).await.unwrap();
                if let Some(sample) = observation.latest() {
                    break sample;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("should receive a sample");

        let mut cache = RenderedRecordCache::default();
        cache.refresh(Some(Arc::clone(&sample)), "[0]");
        assert_eq!(cache.rendered_json_buffer(false).unwrap(), "7.0");
        cache.refresh(Some(Arc::clone(&sample)), "[1]");
        assert_eq!(cache.rendered_json_buffer(false).unwrap(), "9.0");
        assert!(Arc::ptr_eq(cache.sample.as_ref().unwrap(), &sample));
        assert_eq!(
            cache.metadata().unwrap().publication_id,
            format_publication_id(sample.publication_id)
        );
        assert_eq!(
            cache.metadata().unwrap().source_time,
            super::format_time(sample.source_time)
        );

        cache.refresh(Some(Arc::clone(&sample)), "[9]");
        assert!(cache.selection_error.as_ref().unwrap().is_unavailable());
        assert!(cache.rendered_json_buffer(false).is_none());
        assert!(cache.metadata().is_some());
        cache.refresh(Some(Arc::clone(&sample)), "typo");
        assert!(!cache.selection_error.as_ref().unwrap().is_unavailable());
        cache.refresh(Some(Arc::clone(&sample)), "");
        assert_eq!(cache.value, Some(json!([7.0, 9.0])));
        assert!(cache.selection_error.is_none());
    }
}
