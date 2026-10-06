mod history;
mod states;
mod status;

use std::time::Duration;

use eframe::egui::{
    Align, Button, Color32, DragValue, Frame, Label, Layout, Popup, Response, RichText, ScrollArea,
    Tooltip, Ui, emath::format_with_decimals_in_range,
};
use egui_plot::{HoverPosition, Line, Plot, PlotMemory, PlotPoints, Points};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    topic_source::TopicSourceEditor,
};

use history::{PlotHistory, SeriesData, seconds_from};
use states::{StateLane, StateLaneItem, state_at, visible_spans};

const COLORS: [Color32; 6] = [
    Color32::from_rgb(31, 119, 180),
    Color32::from_rgb(255, 127, 14),
    Color32::from_rgb(44, 160, 44),
    Color32::from_rgb(214, 39, 40),
    Color32::from_rgb(148, 103, 189),
    Color32::from_rgb(227, 119, 194),
];

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SavedPlot {
    history_seconds: f64,
    #[serde(deserialize_with = "lenient_items")]
    lines: Vec<SavedLine>,
}

impl Default for SavedPlot {
    fn default() -> Self {
        Self {
            history_seconds: 30.0,
            lines: vec![SavedLine::default()],
        }
    }
}

/// Load saved settings field by field, so an invalid field, for example from
/// an older or hand-edited layout, falls back to its default instead of
/// discarding the whole plot.
fn load_lenient<T: Serialize + DeserializeOwned + Default>(value: &Value) -> T {
    if let Ok(loaded) = T::deserialize(value) {
        return loaded;
    }
    let (Ok(Value::Object(mut merged)), Value::Object(saved)) =
        (serde_json::to_value(T::default()), value)
    else {
        return T::default();
    };
    for (key, field) in saved {
        let default = merged.insert(key.clone(), field.clone());
        if T::deserialize(&Value::Object(merged.clone())).is_err() {
            match default {
                Some(default) => merged.insert(key.clone(), default),
                None => merged.remove(key),
            };
        }
    }
    T::deserialize(&Value::Object(merged)).unwrap_or_default()
}

/// Load each item on its own, so one invalid item keeps the others.
fn lenient_items<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Serialize + DeserializeOwned + Default,
{
    Ok(Vec::<Value>::deserialize(deserializer)?
        .iter()
        .map(load_lenient)
        .collect())
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SavedLine {
    topic: String,
    field_path: String,
    color: Color32,
    visible: bool,
    style: DrawStyle,
}

/// How an item's samples are drawn.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DrawStyle {
    /// Connect consecutive samples, breaking at gaps.
    #[default]
    Line,
    /// Draw each sample as an unconnected point.
    Scatter,
}

impl DrawStyle {
    fn toggled(self) -> Self {
        match self {
            Self::Line => Self::Scatter,
            Self::Scatter => Self::Line,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Line => egui_material_icons::icons::ICON_TIMELINE.codepoint,
            Self::Scatter => egui_material_icons::icons::ICON_SCATTER_PLOT.codepoint,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Line => "Drawn as a line. Click to draw individual samples.",
            Self::Scatter => "Drawn as individual samples. Click to connect them.",
        }
    }
}

impl Default for SavedLine {
    fn default() -> Self {
        Self {
            topic: String::new(),
            field_path: String::new(),
            color: COLORS[0],
            visible: true,
            style: DrawStyle::Line,
        }
    }
}

struct PlotLine {
    id: usize,
    source: TopicSourceEditor,
    color: Color32,
    visible: bool,
    style: DrawStyle,
    data: SeriesData,
}

impl PlotLine {
    fn new(id: usize, saved: SavedLine) -> Self {
        Self {
            id,
            source: TopicSourceEditor::new(saved.topic, saved.field_path),
            color: saved.color,
            visible: saved.visible,
            style: saved.style,
            data: SeriesData::default(),
        }
    }

    fn save(&self) -> SavedLine {
        SavedLine {
            topic: self.source.topic().to_owned(),
            field_path: self.source.field_path().to_owned(),
            color: self.color,
            visible: self.visible,
            style: self.style,
        }
    }
}

pub struct PlotPanel {
    lines: Vec<PlotLine>,
    next_id: usize,
    history_seconds: f64,
    history: PlotHistory,
    paused: bool,
    reset_view: bool,
}

impl Panel for PlotPanel {
    const STORAGE_ID: &'static str = "plot";
    const DISPLAY_NAME: &'static str = "Plot";
    const ICON: &'static str = egui_material_icons::icons::ICON_SHOW_CHART.codepoint;

    fn new(context: PanelCreationContext<'_>) -> Self {
        let saved: SavedPlot = context
            .value
            .map(load_lenient::<SavedPlot>)
            .unwrap_or_default();
        let mut panel = Self {
            next_id: saved.lines.len(),
            lines: saved
                .lines
                .into_iter()
                .enumerate()
                .map(|(id, saved)| PlotLine::new(id, saved))
                .collect(),
            history_seconds: valid_history_seconds(saved.history_seconds),
            history: PlotHistory::default(),
            paused: false,
            reset_view: true,
        };
        panel.history.reconcile(
            &context,
            panel.lines.iter().map(|line| line.source.topic()),
            Duration::from_secs_f64(panel.history_seconds),
        );
        panel
    }

    fn focus_topic(&mut self) {
        // A paused plot still permits inspection; focusing a source should not
        // silently resume collection in the displayed snapshot.
        if !self.paused {
            if self.lines.is_empty() {
                self.add_line();
            }
            self.lines[0].source.request_focus();
        }
    }

    fn header_ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if self.paused { "Resume" } else { "Pause" })
                .on_hover_text("Space toggles pause/resume when the plot is focused.")
                .clicked()
            {
                self.toggle_pause();
            }
            if ui.button("Reset view").clicked() {
                self.reset_view = true;
            }
            ui.label("History");
            ui.add_enabled(
                !self.paused,
                DragValue::new(&mut self.history_seconds)
                    .range(1.0..=600.0)
                    .speed(1.0)
                    .suffix(" s"),
            )
            .on_hover_text(
                "Changing history starts a new buffer. All samples in the selected time window are retained.",
            );
            if ui
                .add_enabled(!self.paused, Button::new("Add item"))
                .clicked()
            {
                self.add_line();
            }
            if self.paused {
                ui.label("Paused: drag or scroll to pan; pinch or Ctrl/Cmd+scroll to zoom.");
            }
        });
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.reset_view = !self.paused;
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        // The backend changes relative-topic namespaces globally. Drop the old
        // snapshots, including paused data, before displaying the new target.
        self.reconcile(&context);
        if !self.paused {
            self.history.refresh();
        }
        let paused = self.paused;
        let mut remove = None;
        ScrollArea::vertical()
            .id_salt("plot-sources")
            .max_height((ui.available_height() * 0.4).max(40.0))
            .show(ui, |ui| {
                for line in &mut self.lines {
                    ui.push_id(line.id, |ui| {
                        let row = item_row(
                            ui,
                            paused,
                            ItemControls {
                                visible: &mut line.visible,
                                color: &mut line.color,
                                style: (!line.data.is_state()).then_some(&mut line.style),
                            },
                            if line.data.is_state() {
                                ItemKind::States
                            } else {
                                ItemKind::Line
                            },
                            |ui| {
                                ui.spacing_mut().text_edit_width = f32::INFINITY;
                                let sample = self.history.latest(line.source.topic());
                                line.source.ui(
                                    ui,
                                    context.backend,
                                    sample.as_ref().map(|record| &record.value),
                                );
                            },
                        );
                        if row.remove {
                            remove = Some(line.id);
                        }
                        self.history.project(
                            line.source.topic(),
                            line.source.field_path(),
                            &mut line.data,
                        );
                        let status = self.history.status(line.source.topic());
                        show_info(ui, &row.info, |ui| {
                            ui.add(Label::new(&status).wrap());
                            if line.data.is_state() {
                                ui.label(format!("{} state intervals", line.data.states.len()));
                            }
                            ui.label(format!("{} gaps", line.data.gaps));
                            if let Some(issue) = &line.data.issue {
                                warning(ui, issue);
                            }
                        });
                    });
                }
            });
        if let Some(id) = remove {
            self.lines.retain(|line| line.id != id);
        }
        self.reconcile(&context);
        if self.lines.is_empty() {
            ui.label("Add an item to plot a numeric topic or field.");
        }

        let end = self.history.end_time();
        let window = end.map(|end| {
            (
                end.saturating_sub(Duration::from_secs_f64(self.history_seconds)),
                end,
            )
        });
        let state_lanes: Vec<_> = self
            .lines
            .iter()
            .filter(|line| line.visible && line.data.is_state())
            .enumerate()
            .map(|(lane, line)| StateLane {
                label: line.source.source(),
                color: line.color,
                spans: window
                    .map(|(start, end)| visible_spans(&line.data.states, start, end).collect())
                    .unwrap_or_default(),
                lane,
            })
            .collect();
        let plot_id = ui.id().with("time-series");
        // Hover labels map positions to lanes with the last drawn bounds.
        let bounds = PlotMemory::load(ui.ctx(), plot_id).map(|memory| *memory.bounds());
        // Grid and crosshair only guide reading; keep them behind the data.
        let guide_color = ui.visuals().text_color().gamma_multiply(0.3);
        let mut plot = Plot::new("time-series")
            .id(plot_id)
            .x_axis_formatter(|mark, _| {
                let decimals = (-mark.step_size.log10().round()).max(0.0) as usize;
                let value = format_with_decimals_in_range(mark.value, decimals..=decimals);
                format!("{value}s")
            })
            .grid_color(guide_color)
            .cursor_color(guide_color)
            .label_formatter(|position| match position {
                HoverPosition::NearDataPoint {
                    plot_name,
                    position,
                    ..
                } => Some(format!(
                    "{plot_name}\n{}\nat {}s",
                    format_with_decimals_in_range(position.y, 0..=6),
                    format_seconds(position.x),
                )),
                HoverPosition::Elsewhere { position } => {
                    let (lane, span) = state_at(&state_lanes, bounds.as_ref()?, *position)?;
                    Some(format!(
                        "{}\n{}\nfrom {}s to {}s",
                        lane.label,
                        span.name,
                        format_seconds(span.x.start),
                        format_seconds(span.x.end),
                    ))
                }
            })
            .allow_double_click_reset(false)
            .allow_drag(self.paused)
            .allow_zoom(self.paused)
            .allow_scroll(self.paused)
            .allow_boxed_zoom(self.paused)
            .allow_axis_zoom_drag(self.paused)
            .height(ui.available_height().max(60.0));
        let reset = std::mem::take(&mut self.reset_view);
        if reset {
            plot = plot.reset();
        }
        let response = plot.show(ui, |plot_ui| {
            if !self.paused || reset {
                plot_ui.set_auto_bounds([false, true]);
                plot_ui.set_plot_bounds_x(-self.history_seconds..=0.0);
            }
            // Draw states first, so lines stay on top.
            for lane in &state_lanes {
                plot_ui.add(StateLaneItem::new(lane, state_lanes.len()));
            }
            let Some((start, end)) = window else {
                return;
            };
            for line in self.lines.iter().filter(|line| line.visible) {
                let label = line.source.source();
                for segment in &line.data.segments {
                    let points: Vec<_> = segment
                        .iter()
                        .filter(|(time, _)| *time >= start && *time <= end)
                        .map(|(time, value)| [seconds_from(*time, end), *value])
                        .collect();
                    match (line.style, points.len()) {
                        (_, 0) => {}
                        (DrawStyle::Scatter, _) => plot_ui.points(
                            Points::new(&label, PlotPoints::new(points))
                                .color(line.color)
                                .radius(2.0),
                        ),
                        (DrawStyle::Line, 1) => plot_ui.points(
                            Points::new(&label, PlotPoints::new(points))
                                .color(line.color)
                                .radius(3.0),
                        ),
                        (DrawStyle::Line, _) => plot_ui
                            .line(Line::new(&label, PlotPoints::new(points)).color(line.color)),
                    }
                }
            }
        });
        if response.response.double_clicked() {
            self.reset_view = true;
            ui.ctx().request_repaint();
        }
    }

    fn save(&self) -> Value {
        serde_json::to_value(SavedPlot {
            history_seconds: self.history_seconds,
            lines: self.lines.iter().map(PlotLine::save).collect(),
        })
        .expect("plot settings are serializable")
    }
}

impl PlotPanel {
    fn add_line(&mut self) {
        let mut line = PlotLine::new(
            self.next_id,
            SavedLine {
                color: COLORS[self.next_id % COLORS.len()],
                ..Default::default()
            },
        );
        line.source.request_focus();
        self.lines.push(line);
        self.next_id += 1;
    }

    fn reconcile(&mut self, context: &PanelUiContext<'_>) {
        self.history_seconds = valid_history_seconds(self.history_seconds);
        if self.history.reconcile(
            context,
            self.lines.iter().map(|line| line.source.topic()),
            Duration::from_secs_f64(self.history_seconds),
        ) {
            self.paused = false;
            self.reset_view = true;
        }
    }
}

fn valid_history_seconds(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(1.0, 600.0)
    } else {
        30.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind {
    Line,
    States,
}

impl ItemKind {
    fn icon(self) -> &'static str {
        match self {
            Self::Line => egui_material_icons::icons::ICON_SHOW_CHART.codepoint,
            Self::States => egui_material_icons::icons::ICON_VIEW_TIMELINE.codepoint,
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Line => "Topic plotted as a line",
            Self::States => "Topic states shown in the background",
        }
    }
}

/// Display settings of an item's row. Items without a drawing style choice
/// show a disabled style button, so all rows keep the same layout.
struct ItemControls<'a> {
    visible: &'a mut bool,
    color: &'a mut Color32,
    style: Option<&'a mut DrawStyle>,
}

struct ItemRow {
    remove: bool,
    info: Response,
}

/// Controls shared by all items: type icon, visibility, color, the source
/// editor, and info and remove buttons. Sources cannot change while paused,
/// but display settings remain editable.
fn item_row(
    ui: &mut Ui,
    paused: bool,
    controls: ItemControls<'_>,
    kind: ItemKind,
    editor: impl FnOnce(&mut Ui),
) -> ItemRow {
    ui.horizontal(|ui| {
        ui.label(kind.icon()).on_hover_text(kind.description());
        ui.checkbox(controls.visible, "").on_hover_text("Show item");
        ui.color_edit_button_srgba(controls.color);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let remove = ui
                .add_enabled(
                    !paused,
                    Button::new(egui_material_icons::icons::ICON_CLOSE.codepoint),
                )
                .on_hover_text("Remove item")
                .clicked();
            let info = ui.button(egui_material_icons::icons::ICON_INFO.codepoint);
            // Drawing style only affects display, so it remains editable while
            // paused, like colors.
            match controls.style {
                Some(style) => {
                    if ui
                        .button(style.icon())
                        .on_hover_text(style.description())
                        .clicked()
                    {
                        *style = style.toggled();
                    }
                }
                None => {
                    ui.add_enabled(false, Button::new(DrawStyle::Line.icon()))
                        .on_disabled_hover_text("Only topic lines can switch to scatter mode.");
                }
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_enabled_ui(!paused, editor);
            });
            ItemRow { remove, info }
        })
        .inner
    })
    .inner
}

/// Show item details on hover or click, fitting the window width.
fn show_info(ui: &Ui, info: &Response, content: impl Fn(&mut Ui)) {
    let max_width = (ui.ctx().content_rect().width()
        - Frame::popup(ui.style()).total_margin().sum().x)
        .max(0.0);
    let show = |ui: &mut Ui| {
        ui.set_max_width(max_width);
        content(ui);
    };
    Tooltip::for_enabled(info)
        .layout(Layout::top_down(Align::Min))
        .width(max_width)
        .show(show);
    Popup::menu(info)
        .layout(Layout::top_down(Align::Min))
        .width(max_width)
        .show(show);
}

fn warning(ui: &mut Ui, text: &str) {
    ui.add(Label::new(RichText::new(text).color(ui.visuals().warn_fg_color)).wrap());
}

fn format_seconds(seconds: f64) -> String {
    format_with_decimals_in_range(seconds, 3..=3)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{DrawStyle, SavedLine, SavedPlot, load_lenient};

    #[test]
    fn layouts_without_a_style_draw_lines() {
        let saved: SavedLine = serde_json::from_value(json!({ "topic": "a" })).unwrap();

        assert!(saved.style == DrawStyle::Line);
    }

    #[test]
    fn style_round_trips_through_layouts() {
        let saved = SavedLine {
            style: DrawStyle::Line.toggled(),
            ..Default::default()
        };
        let value = serde_json::to_value(&saved).unwrap();

        assert_eq!(value["style"], "scatter");
        let restored: SavedLine = serde_json::from_value(value).unwrap();
        assert!(restored.style == DrawStyle::Scatter);
    }

    #[test]
    fn invalid_fields_fall_back_without_discarding_the_plot() {
        let saved: SavedPlot = load_lenient(&json!({
            "history_seconds": "long",
            "lines": [
                { "topic": "a", "color": "red", "style": "scatter" },
                "not a line",
                { "topic": "b", "visible": false },
            ],
        }));

        assert_eq!(saved.history_seconds, 30.0);
        let lines: Vec<_> = saved
            .lines
            .iter()
            .map(|line| (line.topic.as_str(), line.visible, line.style))
            .collect();
        assert!(
            lines
                == [
                    ("a", true, DrawStyle::Scatter),
                    ("", true, DrawStyle::Line),
                    ("b", false, DrawStyle::Line),
                ]
        );
        assert_eq!(saved.lines[0].color, SavedLine::default().color);
    }
}
