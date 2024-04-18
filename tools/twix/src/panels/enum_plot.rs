use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    str::FromStr,
    sync::Arc,
};

use eframe::{
    egui::{
        show_tooltip_at_pointer, Button, ComboBox, Id, Response, RichText, TextStyle, Ui, Widget,
        WidgetText,
    },
    epaint::{Color32, Pos2, RectShape, Rounding, Shape, Stroke, TextShape},
};
use egui_plot::{
    ClosestElem, Cursor, LabelFormatter, Plot, PlotBounds, PlotConfig, PlotGeometry, PlotItem,
    PlotPoint, PlotTransform, PlotUi,
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
}

struct LabeledRect {
    start: f64,
    end: f64,
    bottom: f64,
    top: f64,
    label: String,
    tooltip: Option<String>,
}

impl LabeledRect {
    fn min(&self) -> PlotPoint {
        [self.start, self.bottom].into()
    }

    fn max(&self) -> PlotPoint {
        [self.end, self.top].into()
    }

    fn from_segment(segment: &Segment, index: usize, offset: usize) -> Self {
        Self {
            start: (segment.start + offset) as f64,
            end: (segment.end + offset) as f64,
            bottom: index as f64,
            top: index as f64 + 1.0,
            label: segment.name(),
            tooltip: segment.tooltip(),
        }
    }
}

impl PlotItem for LabeledRect {
    fn shapes(&self, ui: &Ui, transform: &PlotTransform, shapes: &mut Vec<Shape>) {
        let color = PlotItem::color(self);

        let stroke_width = 2.0;
        shapes.push(
            RectShape::new(
                transform
                    .rect_from_values(&self.min(), &self.max())
                    .shrink(stroke_width),
                Rounding::same(4.0),
                color.gamma_multiply(0.5),
                Stroke::new(stroke_width, color),
            )
            .into(),
        );

        let text_margin = 2.0 * stroke_width;
        let left_viewport_edge = transform.frame().left();
        let screenspace_start = transform.position_from_point_x(self.start);
        let screenspace_end = transform.position_from_point_x(self.end);
        let screenspace_top = transform.position_from_point_y(self.top);
        let screenspace_width = screenspace_end - screenspace_start;

        let text = WidgetText::from(&self.label).into_galley(
            ui,
            Some(false),
            screenspace_width,
            TextStyle::Body,
        );

        let text_width = text.rect.width() + 2.0 * text_margin;

        let text_position_x =
            if screenspace_start < left_viewport_edge && left_viewport_edge < screenspace_end {
                (screenspace_end - text_width).min(left_viewport_edge)
            } else {
                screenspace_start
            };

        let text_position = [text_position_x + text_margin, screenspace_top + text_margin].into();

        shapes.push(Shape::LineSegment {
            points: [text.rect.min, text.rect.max],
            stroke: Stroke::new(1.0, Color32::GREEN),
        });
        shapes.push(TextShape::new(text_position, text, Color32::WHITE).into());
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.label
    }

    fn color(&self) -> Color32 {
        color_hash(&self.label)
    }

    fn highlight(&mut self) {}

    fn highlighted(&self) -> bool {
        false
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Rects
    }

    fn bounds(&self) -> PlotBounds {
        PlotBounds::from_min_max([self.start, self.bottom], [self.end, self.top])
    }

    fn id(&self) -> Option<Id> {
        None
    }

    fn find_closest(&self, _point: Pos2, _transform: &PlotTransform) -> Option<ClosestElem> {
        None
    }

    fn on_hover(
        &self,
        _elem: ClosestElem,
        _shapes: &mut Vec<Shape>,
        _cursors: &mut Vec<Cursor>,
        _plot: &PlotConfig<'_>,
        _label_formatter: &LabelFormatter,
    ) {
    }

    fn allow_hover(&self) -> bool {
        false
    }
}

#[derive(Debug, PartialEq)]
enum ViewportMode {
    Full,
    Follow,
    Free,
}

#[derive(Default)]
struct SegmentData {
    output_key: String,
    change_buffer: Option<ChangeBuffer>,
    messages_count: usize,
    last_error: Option<String>,
}

impl SegmentData {
    fn subscribe(&mut self, nao: Arc<Nao>) {
        self.change_buffer = match CyclerOutput::from_str(&self.output_key) {
            Ok(output) => {
                let buffer = nao.subscribe_changes(output);
                Some(buffer)
            }
            Err(error) => {
                error!("Failed to subscribe: {:#}", error);
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
    segment_datas: Vec<SegmentData>,
    scroll_position: f64,
    viewport_width: f64,
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

        let segment_datas = output_keys
            .iter()
            .map(|&output_key| {
                let mut result = SegmentData {
                    output_key: String::from(output_key),
                    ..Default::default()
                };
                result.subscribe(nao.clone());

                result
            })
            .collect();

        Self {
            nao,
            segment_datas,
            scroll_position: 0.0,
            viewport_width: 1.0,
            viewport_mode: ViewportMode::Full,
        }
    }

    fn save(&self) -> Value {
        json!({
            "subscribe_keys": self.segment_datas.iter().map(|segment_data|&segment_data.output_key).collect::<Vec<_>>()
        })
    }
}

impl EnumPlotPanel {
    fn process_user_input(&mut self, plot_ui: &PlotUi) {
        let drag_delta = f64::from(plot_ui.pointer_coordinate_drag_delta().x);

        let cursor_position = plot_ui.pointer_coordinate();
        let scroll_delta = plot_ui.ctx().input(|input| input.smooth_scroll_delta);

        let normalized_cursor_position = cursor_position
            .map_or(0.0, |plot_point| plot_point.x - self.scroll_position)
            / self.viewport_width;

        let previous_viewport_width = self.viewport_width;

        self.viewport_width =
            (self.viewport_width * 0.99f64.powf(f64::from(scroll_delta.y))).max(1.0);

        let zoom_difference = self.viewport_width - previous_viewport_width;
        let zoom_scroll_compensation = zoom_difference * normalized_cursor_position;

        self.scroll_position -= drag_delta
            + self.viewport_width * f64::from(scroll_delta.x) / 400.0
            + zoom_scroll_compensation;
    }

    fn show_plot(&mut self, plot_ui: &mut PlotUi) {
        if plot_ui.response().hovered() {
            self.process_user_input(plot_ui);
        }

        let lines: Vec<_> = self
            .segment_datas
            .iter_mut()
            .map(|segment_data| (segment_data.segments(), segment_data.messages_count))
            .collect();

        let max_message_count = lines
            .iter()
            .map(|(_line, message_count)| *message_count)
            .max()
            .unwrap_or_default();

        let labeled_rect_lines: Vec<Vec<_>> = lines
            .iter()
            .rev()
            .enumerate()
            .map(|(index, (segments, message_count))| {
                let offset = max_message_count - message_count;

                segments
                    .iter()
                    .map(|segment| LabeledRect::from_segment(segment, index, offset))
                    .collect()
            })
            .collect();

        let full_width = max_message_count.max(1) as f64;

        match self.viewport_mode {
            ViewportMode::Full => {
                self.viewport_width = full_width;
                self.scroll_position = 0.0;
            }
            ViewportMode::Follow => {
                self.scroll_position = full_width - self.viewport_width;
            }
            ViewportMode::Free => {}
        }

        let plot_bounds = PlotBounds::from_min_max(
            [self.scroll_position, 0.0],
            [
                self.scroll_position + self.viewport_width,
                self.segment_datas.len() as f64,
            ],
        );
        plot_ui.set_plot_bounds(plot_bounds);

        if plot_ui.response().double_clicked() {
            self.viewport_width = full_width;
            self.scroll_position = 0.0;
        }

        if let Some(hover_position) = plot_ui.response().hover_pos() {
            let plot_hover_position = plot_ui.transform().value_from_position(hover_position);

            let hovered_rect = usize::try_from(plot_hover_position.y as isize)
                .ok()
                .and_then(|index| labeled_rect_lines.get(index))
                .and_then(|labeled_rect_line| {
                    labeled_rect_line.iter().find(|labeled_rect| {
                        labeled_rect.start < plot_hover_position.x
                            && plot_hover_position.x < labeled_rect.end
                    })
                });

            if let Some(hovered_segment) = hovered_rect {
                plot_ui
                    .ctx()
                    .set_cursor_icon(eframe::egui::CursorIcon::Crosshair);

                if let Some(tooltip) = hovered_segment.tooltip.as_ref() {
                    show_tooltip_at_pointer(
                        plot_ui.ctx(),
                        Id::new("enum_plot_segment_tooltip"),
                        |ui| {
                            ui.label(tooltip);
                        },
                    );
                }
            }
        }

        for line in labeled_rect_lines {
            for labeled_rect in line {
                plot_ui.add(labeled_rect)
            }
        }
    }

    fn plot(&mut self, ui: &mut Ui) -> Response {
        const LINE_HEIGHT: f32 = 64.0;

        Plot::new("Jürgen")
            .height(self.segment_datas.len().max(1) as f32 * LINE_HEIGHT)
            .show_y(false)
            .show_x(false)
            .y_axis_width(0)
            .y_grid_spacer(|_| vec![])
            .show_grid(false)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_zoom(false)
            .label_formatter(|_name, _plot_point| String::new())
            .show(ui, |plot_ui| self.show_plot(plot_ui))
            .response
    }
}

impl Widget for &mut EnumPlotPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            self.plot(ui);

            ui.horizontal(|ui| {
                if ui.button("Clear").clicked() {
                    for segment_data in &mut self.segment_datas {
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

            self.segment_datas.retain_mut(|segment_data| {
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
                self.segment_datas.push(SegmentData::default());
            }
        })
        .response
    }
}
