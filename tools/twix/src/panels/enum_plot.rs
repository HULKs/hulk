use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
};

use eframe::{
    egui::{
        show_tooltip_at_pointer, Button, ComboBox, Response, RichText, Sense, TextStyle, Ui,
        Widget, WidgetText,
    },
    emath::{remap, Rangef, RectTransform},
    epaint::{Color32, Rect, Rounding, Shape, Stroke, TextShape, Vec2},
};
use itertools::Itertools;
use log::{error, info};
use serde_json::{json, Value};

use communication::client::CyclerOutput;

use crate::{change_buffer::ChangeBuffer, completion_edit::CompletionEdit, nao::Nao, panel::Panel};

fn color_hash(value: impl Hash) -> Color32 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);

    let hash = hasher.finish();

    let [r, g, b, ..] = hash.to_le_bytes();

    Color32::from_rgb(r, g, b)
}

#[derive(Clone)]
struct Segment {
    start: usize,
    end: usize,
    value: Value,
}

impl Segment {
    fn name(&self) -> String {
        match &self.value {
            Value::Null => "<Null>".into(),
            Value::Bool(value) => value.to_string(),
            Value::Number(_) => "<Number>".into(),
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

    fn render(&self, ui: &mut Ui, offset: usize, index: usize, viewport_transform: &RectTransform) {
        let stroke_color = color_hash(self.name());
        let fill_color = stroke_color.gamma_multiply(0.5);
        let stroke_width = 2.0;

        let rect = Rect::from_min_max(
            [(self.start + offset) as f32, index as f32].into(),
            [(self.end + offset) as f32, (index + 1) as f32].into(),
        );

        if !rect
            .x_range()
            .intersects(viewport_transform.from().x_range())
        {
            return;
        }

        let screenspace_rect = viewport_transform.transform_rect(rect).shrink(stroke_width);

        if ui.rect_contains_pointer(screenspace_rect) {
            if let Some(tooltip) = self.tooltip() {
                show_tooltip_at_pointer(ui.ctx(), "Fridolin".into(), |ui| ui.label(tooltip));
            }
        }

        ui.painter().rect(
            screenspace_rect,
            Rounding::same(4.0),
            fill_color,
            Stroke::new(stroke_width, stroke_color),
        );

        let text_margin = 2.0 * stroke_width;

        let available_text_rect = screenspace_rect
            .intersect(*viewport_transform.to())
            .shrink(text_margin);

        let text = WidgetText::from(&self.name()).into_galley(
            ui,
            Some(false),
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
    output_key: String,
    change_buffer: Option<ChangeBuffer>,
    messages_count: usize,
    last_error: Option<String>,
}

impl SegmentRow {
    fn subscribe(&mut self, nao: Arc<Nao>) {
        self.change_buffer = match CyclerOutput::from_str(&self.output_key) {
            Ok(output) => {
                let buffer = nao.subscribe_changes(output);
                self.last_error = None;
                Some(buffer)
            }
            Err(error) => {
                error!("Failed to subscribe: {:#}", error);
                self.last_error = Some(error.to_string());
                None
            }
        };
    }

    fn show_settings(&mut self, ui: &mut Ui, nao: Arc<Nao>) {
        let subscription_field =
            ui.add(CompletionEdit::outputs(&mut self.output_key, nao.as_ref()));

        if subscription_field.changed() {
            info!("Subscribing: {}", self.output_key);
            if let Some(change_buffer) = self.change_buffer.as_mut() {
                change_buffer.reset();
            }
            self.subscribe(nao);
        }

        if let Some(error) = self.last_error.as_ref() {
            ui.colored_label(Color32::RED, error);
        }
    }

    fn segments(&mut self) -> Vec<Segment> {
        self.change_buffer
            .as_ref()
            .and_then(|change_buffer| match change_buffer.get_buffered() {
                Ok(change_buffer_update) => {
                    let mut segments = Vec::new();

                    self.messages_count = change_buffer_update.message_count;

                    for (start, end) in change_buffer_update.updates.iter().tuple_windows() {
                        segments.push(Segment {
                            start: start.message_number,
                            end: end.message_number,
                            value: start.value.clone(),
                        });
                    }

                    if let Some(last_change) = change_buffer_update.updates.last() {
                        segments.push(Segment {
                            start: last_change.message_number,
                            end: self.messages_count,
                            value: last_change.value.clone(),
                        });
                    }

                    Some(segments)
                }
                Err(error) => {
                    self.last_error = Some(error);

                    None
                }
            })
            .unwrap_or_default()
    }

    fn clear(&mut self) {
        if let Some(change_buffer) = self.change_buffer.as_ref() {
            change_buffer.clear();
        }
        self.messages_count = 0;
    }
}

pub struct EnumPlotPanel {
    nao: Arc<Nao>,
    segment_rows: Vec<SegmentRow>,
    x_range: Rangef,
    viewport_mode: ViewportMode,
}

impl Panel for EnumPlotPanel {
    const NAME: &'static str = "Enum Plot";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let output_keys: Vec<_> = value
            .and_then(|value| value.get("subscribe_keys"))
            .and_then(|value| value.as_array())
            .map(|values| values.iter().flat_map(|value| value.as_str()).collect())
            .unwrap_or_default();

        let segment_rows = output_keys
            .iter()
            .map(|&output_key| {
                let mut result = SegmentRow {
                    output_key: String::from(output_key),
                    ..Default::default()
                };
                result.subscribe(nao.clone());

                result
            })
            .collect();

        Self {
            nao,
            segment_rows,
            x_range: Rangef::new(0.0, 1000.0),
            viewport_mode: ViewportMode::Follow,
        }
    }

    fn save(&self) -> Value {
        json!({
            "subscribe_keys": self.segment_rows.iter().map(|segment_data|&segment_data.output_key).collect::<Vec<_>>()
        })
    }
}

impl EnumPlotPanel {
    fn interact(&mut self, response: &Response, ui: &mut Ui, max_message_count: usize) {
        const SCROLL_THRESHOLD: f32 = 1.0;

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

            let new_viewport_width =
                (previous_viewport_width * 0.99f32.powf(scroll_delta.y)).max(1.0);

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
                self.x_range = Rangef::new(0.0, max_message_count as f32);
            }
            ViewportMode::Follow => {
                self.x_range = Rangef::new(
                    max_message_count as f32 - viewport_width,
                    max_message_count as f32,
                );
            }
            ViewportMode::Free => {
                self.x_range = Rangef::new(scroll_position, scroll_position + viewport_width);
            }
        }
    }

    fn render(&mut self, ui: &mut Ui) -> Response {
        const LINE_HEIGHT: f32 = 64.0;

        let desired_size = Vec2::new(
            ui.available_width(),
            self.segment_rows.len().max(1) as f32 * LINE_HEIGHT,
        );

        let lines: Vec<_> = self
            .segment_rows
            .iter_mut()
            .map(|segment_data| (segment_data.segments(), segment_data.messages_count))
            .collect();

        let max_message_count = lines
            .iter()
            .map(|(_line, message_count)| *message_count)
            .max()
            .unwrap_or_default();

        let (frame, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        self.interact(&response, ui, max_message_count);

        let viewport_rect = Rect::from_x_y_ranges(
            self.x_range,
            Rangef::new(0.0, self.segment_rows.len() as f32),
        );

        let viewport_transform = RectTransform::from_to(viewport_rect, response.rect);

        ui.scope(|ui| {
            ui.set_clip_rect(frame);
            ui.painter()
                .rect_filled(frame, Rounding::ZERO, Color32::BLACK);

            for (index, (segments, message_count)) in lines.iter().enumerate() {
                let offset = max_message_count - message_count;

                for segment in segments {
                    segment.render(ui, offset, index, &viewport_transform);
                }
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
                if ui.button("Clear").clicked() {
                    for segment_data in &mut self.segment_rows {
                        segment_data.clear();
                    }
                }

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

            self.segment_rows.retain_mut(|segment_data| {
                ui.horizontal(|ui| {
                    let delete_button = ui.add(
                        Button::new(RichText::new("❌").color(Color32::WHITE).strong())
                            .fill(Color32::RED),
                    );

                    segment_data.show_settings(ui, self.nao.clone());
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
