mod history;

#[cfg(test)]
mod tests;

use std::time::Duration;

use eframe::egui::{Button, Color32, DragValue, ScrollArea, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    topic_source::TopicSourceEditor,
};

use history::{PlotHistory, SeriesData, seconds_from};

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

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct SavedLine {
    topic: String,
    field_path: String,
    color: Color32,
    visible: bool,
}

impl Default for SavedLine {
    fn default() -> Self {
        Self {
            topic: String::new(),
            field_path: String::new(),
            color: COLORS[0],
            visible: true,
        }
    }
}

struct PlotLine {
    id: usize,
    source: TopicSourceEditor,
    color: Color32,
    visible: bool,
    data: SeriesData,
}

impl PlotLine {
    fn new(id: usize, saved: SavedLine) -> Self {
        Self {
            id,
            source: TopicSourceEditor::new(saved.topic, saved.field_path),
            color: saved.color,
            visible: saved.visible,
            data: SeriesData::default(),
        }
    }

    fn save(&self) -> SavedLine {
        SavedLine {
            topic: self.source.topic().to_owned(),
            field_path: self.source.field_path().to_owned(),
            color: self.color,
            visible: self.visible,
        }
    }

    fn label(&self) -> String {
        let topic = self.source.topic();
        let path = self.source.field_path();
        if path.is_empty() {
            topic.to_owned()
        } else if path.starts_with('[') || path.starts_with("::") {
            format!("{topic}{path}")
        } else {
            format!("{topic}.{path}")
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
            .and_then(|value| serde_json::from_value(value.clone()).ok())
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
                .clicked()
            {
                self.paused = !self.paused;
                self.reset_view = !self.paused;
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
                "Changing history starts a new buffer. Up to 4096 samples per topic are retained.",
            );
            if ui
                .add_enabled(!self.paused, Button::new("Add line"))
                .clicked()
            {
                self.add_line();
            }
            if self.paused {
                ui.label("Paused: drag to pan, scroll to zoom.");
            }
        });
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        // The backend changes relative-topic namespaces globally. Drop the old
        // snapshots, including paused data, before displaying the new target.
        self.reconcile(&context);
        if !self.paused {
            self.history.refresh();
        }
        let mut remove = None;
        ScrollArea::vertical()
            .id_salt("plot-sources")
            .max_height((ui.available_height() * 0.4).max(40.0))
            .show(ui, |ui| {
                for line in &mut self.lines {
                    ui.push_id(line.id, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.checkbox(&mut line.visible, "")
                                .on_hover_text("Show line");
                            ui.color_edit_button_srgba(&mut line.color);
                            ui.add_enabled_ui(!self.paused, |ui| {
                                ui.spacing_mut().text_edit_width =
                                    (ui.available_width() - 65.0).max(80.0);
                                let sample = self.history.latest(line.source.topic());
                                line.source.ui(
                                    ui,
                                    context.backend,
                                    sample.as_ref().map(|record| &record.value),
                                );
                                if ui.small_button("Remove").clicked() {
                                    remove = Some(line.id);
                                }
                            });
                        });
                        self.history.project(
                            line.source.topic(),
                            line.source.field_path(),
                            &mut line.data,
                        );
                        let status = self.history.status(line.source.topic());
                        if let Some(issue) = &line.data.issue {
                            ui.colored_label(ui.visuals().warn_fg_color, issue)
                                .on_hover_text(status);
                        } else if line.data.segments.is_empty() {
                            ui.weak(status);
                        } else {
                            ui.add(
                                eframe::egui::Label::new(format!(
                                    "{status} · {} gaps",
                                    line.data.gaps
                                ))
                                .truncate(),
                            )
                            .on_hover_text(status);
                        }
                    });
                }
            });
        if let Some(id) = remove {
            self.lines.retain(|line| line.id != id);
        }
        self.reconcile(&context);
        if self.lines.is_empty() {
            ui.label("Add a line to plot a numeric topic or field.");
        }

        let end = self.history.end_time();
        let mut plot = Plot::new(ui.id().with("time-series"))
            .legend(Legend::default())
            .x_axis_label("Seconds relative to newest sample (source time)")
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
        plot.show(ui, |plot_ui| {
            if !self.paused || reset {
                plot_ui.set_auto_bounds([false, true]);
                plot_ui.set_plot_bounds_x(-self.history_seconds..=0.0);
            }
            let Some(end) = end else {
                return;
            };
            let start = end.saturating_sub(Duration::from_secs_f64(self.history_seconds));
            for line in self.lines.iter().filter(|line| line.visible) {
                let label = line.label();
                for segment in &line.data.segments {
                    let points: Vec<_> = segment
                        .iter()
                        .filter(|(time, _)| *time >= start && *time <= end)
                        .map(|(time, value)| [seconds_from(*time, end), *value])
                        .collect();
                    match points.len() {
                        0 => {}
                        1 => plot_ui.points(
                            Points::new(&label, PlotPoints::new(points))
                                .color(line.color)
                                .radius(3.0),
                        ),
                        _ => plot_ui
                            .line(Line::new(&label, PlotPoints::new(points)).color(line.color)),
                    }
                }
            }
        });
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
