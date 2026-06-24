use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    iter::once,
    ops::Range,
    sync::Arc,
    time::{Duration, SystemTime},
};

use eframe::{
    egui::{
        Align2, Button, ComboBox, FontId, Label, PopupAnchor, Response, RichText, Sense,
        StrokeKind, TextStyle, TextWrapMode, Tooltip, Ui, Widget, WidgetText,
    },
    emath::{Rangef, RectTransform, remap},
    epaint::{Color32, CornerRadius, Rect, Shape, Stroke, TextShape, Vec2},
};
use itertools::Itertools;
use ros_z::time::Time;
use ros_z_debug::RetentionPolicy;
use serde_json::{Value, json};

use crate::{
    backend::{TwixBackend, retained_subscription::DynamicSubscription},
    panel::{Panel, PanelCreationContext},
    topic_completion_edit::TopicCompletionEdit,
};

const DEFAULT_RETENTION_WINDOW: Duration = Duration::from_secs(10);

fn enum_plot_retention(duration: Duration) -> RetentionPolicy {
    RetentionPolicy::time_window(duration).unwrap_or(RetentionPolicy::LatestOnly)
}

fn color_hash(value: impl Hash) -> Color32 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);

    let hash = hasher.finish();

    let [r, g, b, ..] = hash.to_le_bytes();

    Color32::from_rgb(r, g, b)
}

#[derive(Debug, Clone)]
struct Segment {
    start: f32,
    end: f32,
    value: Value,
}

#[derive(Clone, Debug, PartialEq)]
struct EnumTransition {
    timestamp: SystemTime,
    value: Value,
}

fn enum_transitions(samples: impl IntoIterator<Item = (SystemTime, Value)>) -> Vec<EnumTransition> {
    let mut transitions = Vec::new();
    for (timestamp, value) in samples {
        if transitions
            .last()
            .is_none_or(|transition: &EnumTransition| transition.value != value)
        {
            transitions.push(EnumTransition { timestamp, value });
        }
    }
    transitions
}

impl Segment {
    fn name(&self) -> String {
        match &self.value {
            Value::Null => "<Null>".into(),
            Value::Bool(value) => value.to_string(),
            Value::Number(number) => number.to_string(),
            Value::String(string) => string.clone(),
            Value::Array(_) => "<Array>".into(),
            Value::Object(map) => {
                if map.keys().len() == 1 {
                    map.keys().next().unwrap().clone()
                } else {
                    "<Object>".into()
                }
            }
        }
    }

    fn tooltip(&self) -> Option<String> {
        match &self.value {
            Value::Number(number) => Some(number.to_string()),
            Value::String(string) => Some(string.clone()),
            Value::Object(map) => {
                if map.keys().len() == 1 {
                    let (key, value) = map.iter().next().unwrap();
                    Some(format!("{key} {value:#}"))
                } else {
                    Some(format!("{map:#?}"))
                }
            }
            Value::Array(array) => Some(format!("{array:#?}")),
            _ => None,
        }
    }

    fn render(&self, ui: &mut Ui, index: usize, viewport_transform: &RectTransform) {
        let stroke_color = color_hash(self.name());
        let fill_color = stroke_color.gamma_multiply(0.5);
        let stroke_width = 2.0;

        let x_min = self.start;
        let x_max = self.end;
        let y_min = index as f32;
        let y_max = (index + 1) as f32;
        let rect = Rect::from_x_y_ranges(x_min..=x_max, y_min..=y_max);

        let is_segment_in_viewport = rect
            .x_range()
            .intersects(viewport_transform.from().x_range());
        if !is_segment_in_viewport {
            return;
        }

        let screenspace_rect = viewport_transform.transform_rect(rect).shrink(stroke_width);

        if ui.rect_contains_pointer(screenspace_rect)
            && let Some(tooltip) = self.tooltip()
        {
            Tooltip::always_open(
                ui.ctx().clone(),
                ui.layer_id(),
                "Fridolin".into(),
                PopupAnchor::Pointer,
            )
            .gap(12.0)
            .show(|ui| {
                ui.add(Label::new(tooltip).wrap_mode(TextWrapMode::Extend));
            });
        }

        ui.painter().rect(
            screenspace_rect,
            CornerRadius::same(4),
            fill_color,
            Stroke::new(stroke_width, stroke_color),
            StrokeKind::Middle,
        );

        let text_margin = 2.0 * stroke_width;

        let available_text_rect = screenspace_rect
            .intersect(*viewport_transform.to())
            .shrink(text_margin);

        let text = WidgetText::from(&self.name()).into_galley(
            ui,
            Some(eframe::egui::TextWrapMode::Truncate),
            available_text_rect.width(),
            TextStyle::Body,
        );

        ui.painter_at(available_text_rect)
            .add(Shape::from(TextShape::new(
                available_text_rect.left_top(),
                text,
                Color32::WHITE,
            )));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ViewportMode {
    Full,
    Follow,
    Free,
}

#[derive(Default)]
struct SegmentRow {
    topic: String,
    subscription: Option<DynamicSubscription>,
}

impl SegmentRow {
    fn subscribe(&mut self, backend: Arc<TwixBackend>, retention_window: Duration) {
        self.subscription =
            if self.topic.is_empty() {
                None
            } else {
                Some(backend.subscribe_json_retained(
                    self.topic.clone(),
                    enum_plot_retention(retention_window),
                ))
            };
    }

    fn show_settings(
        &mut self,
        ui: &mut Ui,
        backend: Arc<TwixBackend>,
        retention_window: Duration,
    ) {
        let subscription_field = ui.add(TopicCompletionEdit::namespace_topics(
            ui.auto_id_with("enum-plot"),
            backend.topic_catalog(),
            &mut self.topic,
        ));

        if subscription_field.changed() {
            self.subscribe(backend, retention_window);
        }

        if let Some(message) = self.diagnostic_message() {
            ui.colored_label(Color32::RED, message);
        }
    }

    fn diagnostic_message(&self) -> Option<String> {
        let subscription = self.subscription.as_ref()?;
        subscription.diagnostic_message()
    }

    fn set_retention(&self, retention_window: Duration) {
        if let Some(subscription) = &self.subscription {
            subscription.set_retention(enum_plot_retention(retention_window));
        }
    }

    fn samples(&self) -> Vec<(Time, Value)> {
        self.subscription
            .as_ref()
            .map(|subscription| subscription.window_json(Time::zero(), Time::from_nanos(i64::MAX)))
            .unwrap_or_default()
    }

    fn segments(
        &self,
        samples: &[(Time, Value)],
        timestamp_range: &Range<SystemTime>,
    ) -> Vec<Segment> {
        let transitions = enum_transitions(
            samples
                .iter()
                .cloned()
                .map(|(timestamp, value)| (timestamp.to_wallclock(), value)),
        );
        let row_end = samples
            .iter()
            .map(|(timestamp, _)| timestamp.to_wallclock())
            .max()
            .unwrap_or(timestamp_range.end);
        let end_transition = EnumTransition {
            timestamp: row_end,
            value: Value::Null,
        };

        transitions
            .iter()
            .chain(once(&end_transition))
            .tuple_windows()
            .map(|(start, end)| Segment {
                start: start
                    .timestamp
                    .duration_since(timestamp_range.start)
                    .unwrap_or_default()
                    .as_secs_f32(),
                end: end
                    .timestamp
                    .duration_since(timestamp_range.start)
                    .unwrap_or_default()
                    .as_secs_f32(),
                value: start.value.clone(),
            })
            .collect()
    }
}

pub struct EnumPlotPanel {
    backend: Arc<TwixBackend>,
    segment_rows: Vec<SegmentRow>,
    x_range: Rangef,
    viewport_mode: ViewportMode,
}

impl<'a> Panel<'a> for EnumPlotPanel {
    const NAME: &'static str = "Enum Plot";

    fn new(context: PanelCreationContext) -> Self {
        let output_keys: Vec<_> = context
            .value
            .and_then(|value| value.get("topics").or_else(|| value.get("paths")))
            .and_then(Value::as_array)
            .map(|values| values.iter().flat_map(Value::as_str).collect())
            .unwrap_or_default();

        let segment_rows = output_keys
            .iter()
            .map(|&output_key| {
                let mut result = SegmentRow {
                    topic: output_key.to_string(),
                    ..Default::default()
                };
                result.subscribe(context.backend.clone(), DEFAULT_RETENTION_WINDOW);

                result
            })
            .collect();

        Self {
            backend: context.backend,
            segment_rows,
            x_range: Rangef::new(-3.0, 0.0),
            viewport_mode: ViewportMode::Follow,
        }
    }

    fn save(&self) -> Value {
        let topics = self
            .segment_rows
            .iter()
            .map(|segment_data| &segment_data.topic)
            .collect::<Vec<_>>();
        json!({
            "topics": topics
        })
    }
}

impl EnumPlotPanel {
    fn retention_window(&self) -> Duration {
        DEFAULT_RETENTION_WINDOW.max(Duration::from_secs_f32(self.x_range.span().max(0.0)))
    }

    fn interact(&mut self, response: &Response, ui: &mut Ui, timestamp_range: &Range<SystemTime>) {
        const SCROLL_THRESHOLD: f32 = 1.0;
        const MINIMUM_VISIBLE_DURATION: Duration = Duration::from_millis(10);

        let (scroll_position, viewport_width) = if response.contains_pointer() {
            let drag_delta = response.drag_delta();
            let drag_offset = self.x_range.span() * (-drag_delta.x / response.rect.width());

            let (cursor_position, scroll_delta, delta_time) = ui.input(|input| {
                if let Some(hover_position) = input.pointer.hover_pos() {
                    (hover_position, input.smooth_scroll_delta, input.unstable_dt)
                } else {
                    (response.rect.center(), Vec2::ZERO, input.unstable_dt)
                }
            });

            let normalized_cursor_position = remap(
                cursor_position.x,
                response.rect.x_range(),
                Rangef::new(0.0, 1.0),
            );

            let previous_viewport_width = self.x_range.span();
            let previous_scroll_position = self.x_range.min;

            let scroll_offset = -previous_viewport_width * scroll_delta.x / 400.0;

            let new_viewport_width = (previous_viewport_width * 0.99f32.powf(scroll_delta.y))
                .max(MINIMUM_VISIBLE_DURATION.as_secs_f32());

            let zoom_difference = new_viewport_width - previous_viewport_width;
            let zoom_scroll_compensation = -zoom_difference * normalized_cursor_position;

            let scroll_offset = drag_offset + scroll_offset;

            self.viewport_mode = match &self.viewport_mode {
                ViewportMode::Full if scroll_delta.y.abs() / delta_time > SCROLL_THRESHOLD => {
                    ViewportMode::Follow
                }
                ViewportMode::Full | ViewportMode::Follow
                    if scroll_delta.x.abs() / delta_time > SCROLL_THRESHOLD
                        || drag_delta.x != 0.0 =>
                {
                    ViewportMode::Free
                }

                other => *other,
            };

            (
                previous_scroll_position + scroll_offset + zoom_scroll_compensation,
                new_viewport_width,
            )
        } else {
            (self.x_range.min, self.x_range.span())
        };

        if response.double_clicked() {
            self.viewport_mode = match self.viewport_mode {
                ViewportMode::Full | ViewportMode::Free => ViewportMode::Follow,
                ViewportMode::Follow => ViewportMode::Full,
            }
        }

        match self.viewport_mode {
            ViewportMode::Full => {
                self.x_range = Rangef::new(
                    0.0,
                    timestamp_range
                        .end
                        .duration_since(timestamp_range.start)
                        .unwrap_or_default()
                        .as_secs_f32(),
                );
            }
            ViewportMode::Follow => {
                let timestamps_span = timestamp_range
                    .end
                    .duration_since(timestamp_range.start)
                    .unwrap_or_default()
                    .as_secs_f32();

                self.x_range = Rangef::new(timestamps_span - viewport_width, timestamps_span);
            }
            ViewportMode::Free => {
                self.x_range = Rangef::new(scroll_position, scroll_position + viewport_width);
            }
        }
    }

    fn render(&mut self, ui: &mut Ui) -> Response {
        const LINE_HEIGHT: f32 = 64.0;
        let retention_window = self.retention_window();
        for row in &self.segment_rows {
            row.set_retention(retention_window);
        }
        let samples = self
            .segment_rows
            .iter()
            .map(SegmentRow::samples)
            .collect_vec();
        let start = samples
            .iter()
            .flat_map(|samples| {
                samples
                    .iter()
                    .map(|(timestamp, _)| timestamp.to_wallclock())
            })
            .min();
        let end = samples
            .iter()
            .flat_map(|samples| {
                samples
                    .iter()
                    .map(|(timestamp, _)| timestamp.to_wallclock())
            })
            .max();

        let desired_size = Vec2::new(
            ui.available_width(),
            self.segment_rows.len().max(1) as f32 * LINE_HEIGHT,
        );

        let (frame, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        ui.scope(|ui| {
            ui.set_clip_rect(frame);
            ui.painter()
                .rect_filled(frame, CornerRadius::ZERO, Color32::BLACK);

            if let (Some(start), Some(end)) = (start, end) {
                let timestamp_range = Range { start, end };
                let lines = self
                    .segment_rows
                    .iter()
                    .zip(samples.iter())
                    .map(|(segment_row, samples)| segment_row.segments(samples, &timestamp_range))
                    .collect_vec();

                self.interact(&response, ui, &timestamp_range);

                let viewport_rect = Rect::from_x_y_ranges(
                    self.x_range,
                    Rangef::new(0.0, self.segment_rows.len() as f32),
                );

                let viewport_transform = RectTransform::from_to(viewport_rect, response.rect);

                for (index, segments) in lines.iter().enumerate() {
                    for segment in segments {
                        segment.render(ui, index, &viewport_transform);
                    }
                }
            } else {
                ui.painter().text(
                    frame.center(),
                    Align2::CENTER_CENTER,
                    "(nothing to show)",
                    FontId::default(),
                    Color32::GRAY,
                );
            }
        });

        response
    }
}

impl Widget for &mut EnumPlotPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            self.render(ui);

            ui.horizontal(|ui| {
                ui.label("Viewport mode:");
                ComboBox::new("viewport_mode", "")
                    .selected_text(format!("{:?}", self.viewport_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.viewport_mode, ViewportMode::Full, "Full");
                        ui.selectable_value(
                            &mut self.viewport_mode,
                            ViewportMode::Follow,
                            "Follow",
                        );
                        ui.selectable_value(&mut self.viewport_mode, ViewportMode::Free, "Free");
                    });
            });

            let backend = self.backend.clone();
            let retention_window = self.retention_window();
            self.segment_rows.retain_mut(|segment_data| {
                ui.horizontal(|ui| {
                    let delete_button = ui.add(
                        Button::new(RichText::new("❌").color(Color32::WHITE).strong())
                            .fill(Color32::RED),
                    );
                    segment_data.show_settings(ui, backend.clone(), retention_window);
                    !delete_button.clicked()
                })
                .inner
            });

            if ui.button("✚").clicked() {
                self.segment_rows.push(SegmentRow::default());
            }
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use serde_json::json;

    use super::*;

    #[test]
    fn enum_transitions_skip_consecutive_equal_values() {
        let base = SystemTime::UNIX_EPOCH;
        let samples = vec![
            (base, json!("initial")),
            (base + Duration::from_secs(1), json!("initial")),
            (base + Duration::from_secs(2), json!("ready")),
        ];

        let transitions = enum_transitions(samples);

        assert_eq!(
            transitions
                .iter()
                .map(|transition| transition.value.clone())
                .collect::<Vec<_>>(),
            vec![json!("initial"), json!("ready")]
        );
    }

    #[test]
    fn row_segments_end_at_row_last_sample_not_global_end() {
        let stopped_row = SegmentRow::default();
        let active_row = SegmentRow::default();
        let timestamp_range = Range {
            start: SystemTime::UNIX_EPOCH,
            end: SystemTime::UNIX_EPOCH + Duration::from_secs(10),
        };
        let stopped_samples = vec![(Time::from_nanos(2_000_000_000), json!("stopped"))];
        let active_samples = vec![(Time::from_nanos(10_000_000_000), json!("active"))];

        let stopped_segments = stopped_row.segments(&stopped_samples, &timestamp_range);
        let active_segments = active_row.segments(&active_samples, &timestamp_range);

        assert_eq!(stopped_segments.len(), 1);
        assert_eq!(active_segments.len(), 1);
        assert_eq!(stopped_segments[0].start, 2.0);
        assert_eq!(stopped_segments[0].end, 2.0);
        assert_eq!(active_segments[0].end, 10.0);
    }
}
