use std::{sync::Arc, time::Duration};

use color_eyre::eyre::{Context, OptionExt};
use eframe::{
    egui::{Button, CollapsingHeader, DragValue, Response, TextEdit, TextStyle, Ui, Widget},
    epaint::Color32,
};
use egui_plot::{Line, MarkerShape, Plot as EguiPlot, PlotPoints, Points};
use itertools::Itertools;
use mlua::{Function, Lua, LuaSerdeExt};
use ros_z::time::Time;
use ros_z_debug::RetentionPolicy;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, to_string_pretty};

use crate::{
    backend::{TwixBackend, retained_subscription::DynamicSubscription},
    panel::{Panel, PanelCreationContext},
    topic_completion_edit::TopicCompletionEdit,
};

const DEFAULT_LINE_COLORS: &[Color32] = &[
    Color32::from_rgb(31, 119, 180),
    Color32::from_rgb(255, 127, 14),
    Color32::from_rgb(44, 160, 44),
    Color32::from_rgb(214, 39, 40),
    Color32::from_rgb(148, 103, 189),
    Color32::from_rgb(140, 86, 75),
    Color32::from_rgb(227, 119, 194),
    Color32::from_rgb(127, 127, 127),
    Color32::from_rgb(188, 189, 34),
    Color32::from_rgb(23, 190, 207),
];
const DEFAULT_LUA_TEXT: &str = "function (value)\n  return value\nend";

fn plot_retention(history: Duration) -> RetentionPolicy {
    RetentionPolicy::time_window(history).unwrap_or(RetentionPolicy::LatestOnly)
}

fn samples_for_retention(
    retention: RetentionPolicy,
    latest_sample: impl FnOnce() -> Option<(Time, Value)>,
    window_samples: impl FnOnce() -> Vec<(Time, Value)>,
) -> Vec<(Time, Value)> {
    match retention {
        RetentionPolicy::LatestOnly => latest_sample().into_iter().collect(),
        RetentionPolicy::TimeWindow(_) => {
            let samples = window_samples();
            if samples.is_empty() {
                latest_sample().into_iter().collect()
            } else {
                samples
            }
        }
        _ => window_samples(),
    }
}

#[derive(Serialize, Deserialize)]
struct LineData {
    #[serde(alias = "path")]
    topic: String,
    #[serde(skip)]
    subscription: Option<DynamicSubscription>,
    color: Color32,
    #[serde(skip)]
    #[serde(default = "LineData::create_lua")]
    lua: Lua,
    lua_text: String,
    #[serde(skip)]
    lua_error: Option<String>,
    #[serde(skip)]
    is_highlighted: bool,
    #[serde(skip)]
    is_hidden: bool,
    #[serde(skip)]
    show_scatter: bool,
}

impl LineData {
    fn create_lua() -> Lua {
        Lua::new()
    }

    fn install_lua_function(&self, lua_text: &str) -> mlua::Result<()> {
        let function = self.lua.load(lua_text).eval::<Function>()?;
        self.lua.globals().set("conversion_function", function)
    }

    fn set_lua(&mut self) {
        self.lua_error = match self.install_lua_function(&self.lua_text) {
            Ok(()) => None,
            Err(error) => {
                let error = format!("{error:#}");
                match self.install_lua_function(DEFAULT_LUA_TEXT) {
                    Ok(()) => Some(error),
                    Err(fallback_error) => Some(format!(
                        "{error}\nfailed to install fallback conversion: {fallback_error:#}"
                    )),
                }
            }
        };
    }

    fn new(color: Color32) -> Self {
        let lua = LineData::create_lua();
        let lua_text = DEFAULT_LUA_TEXT.to_string();

        let mut line_data = Self {
            topic: String::new(),
            subscription: None,
            color,
            lua,
            lua_text,
            lua_error: None,
            is_highlighted: false,
            is_hidden: false,
            show_scatter: false,
        };

        line_data.set_lua();
        line_data
    }

    fn set_highlighted(&mut self, is_highlighted: bool) {
        self.is_highlighted = is_highlighted
    }

    fn samples(&self, buffer_history: Duration) -> Vec<(Time, Value)> {
        let Some(subscription) = self.subscription.as_ref() else {
            return Vec::new();
        };

        samples_for_retention(
            plot_retention(buffer_history),
            || subscription.latest_json_with_timestamp(),
            || subscription.window_json(Time::zero(), Time::from_nanos(i64::MAX)),
        )
    }

    fn plot(&self, samples: &[(Time, Value)], latest_timestamp: Option<Time>) -> PlotPoints<'_> {
        let Some(latest_timestamp) = latest_timestamp else {
            return PlotPoints::default();
        };
        let Ok(lua_function) = self.lua.globals().get::<Function>("conversion_function") else {
            return PlotPoints::default();
        };

        PlotPoints::from_iter(samples.iter().map(|(timestamp, value)| {
            let value = lua_function
                .call(self.lua.to_value(value))
                .unwrap_or(f64::NAN);
            [
                -latest_timestamp.duration_since(*timestamp).as_secs_f64(),
                value,
            ]
        }))
    }

    fn show_settings(
        &mut self,
        ui: &mut Ui,
        id: usize,
        backend: &TwixBackend,
        buffer_history: Duration,
    ) {
        ui.horizontal_top(|ui| {
            let subscription_field = ui.add(TopicCompletionEdit::namespace_topics(
                ui.id().with(id).with("plot-panel"),
                backend.topic_catalog(),
                &mut self.topic,
            ));
            self.set_highlighted(subscription_field.hovered());
            if subscription_field.changed() {
                self.subscription = if self.topic.is_empty() {
                    None
                } else {
                    Some(backend.subscribe_json_retained(
                        self.topic.clone(),
                        plot_retention(buffer_history),
                    ))
                };
            }

            ui.color_edit_button_srgba(&mut self.color);

            let id_salt = ui.id().with("conversion_collapse").with(id);
            CollapsingHeader::new("Conversion Function")
                .id_salt(id_salt)
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if let Some(message) = self
                            .subscription
                            .as_ref()
                            .and_then(DynamicSubscription::diagnostic_message)
                        {
                            ui.colored_label(Color32::RED, message);
                        }
                        let latest_value = self
                            .subscription
                            .as_ref()
                            .ok_or_eyre("no subscription")
                            .and_then(|subscription| {
                                subscription.latest_json().ok_or_eyre("no value")
                            });

                        let pretty_json = match &latest_value {
                            Ok(value) => to_string_pretty(value)
                                .wrap_err("failed to serialize value")
                                .unwrap_or_else(|error| format!("{error:#}")),
                            Err(error) => format!("{error:#}"),
                        };
                        ui.label(pretty_json);
                        let code_edit = TextEdit::multiline(&mut self.lua_text)
                            .font(TextStyle::Monospace)
                            .code_editor()
                            .lock_focus(true);
                        if ui.add(code_edit).changed() {
                            self.set_lua();
                        }
                        if let Some(error) = &self.lua_error {
                            ui.colored_label(Color32::RED, error);
                        } else if let Ok(value) = &latest_value {
                            let value = self
                                .lua
                                .globals()
                                .get::<Function>("conversion_function")
                                .and_then(|function| {
                                    self.lua
                                        .to_value(value)
                                        .and_then(|value| function.call::<f64>(value))
                                });
                            match value {
                                Ok(value) => {
                                    ui.label(value.to_string());
                                }
                                Err(error) => {
                                    ui.colored_label(Color32::RED, error.to_string());
                                }
                            }
                        }
                    });
                });
        });
    }
}

pub struct PlotPanel {
    lines: Vec<LineData>,
    buffer_history: Duration,
    backend: Arc<TwixBackend>,
}

impl<'a> Panel<'a> for PlotPanel {
    const NAME: &'static str = "Plot";

    fn new(context: PanelCreationContext) -> Self {
        const DEFAULT_BUFFER_HISTORY: Duration = Duration::from_secs(10);

        let lines = context
            .value
            .and_then(|value| value["lines"].as_array())
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|line_data| {
                        let mut line_data =
                            serde_json::from_value::<LineData>(line_data.clone()).ok()?;
                        line_data.set_lua();
                        if !line_data.topic.is_empty() {
                            let subscription = context.backend.subscribe_json_retained(
                                line_data.topic.clone(),
                                plot_retention(DEFAULT_BUFFER_HISTORY),
                            );
                            line_data.subscription = Some(subscription);
                        }
                        Some(line_data)
                    })
                    .collect_vec()
            })
            .unwrap_or_default();

        PlotPanel {
            lines,
            buffer_history: DEFAULT_BUFFER_HISTORY,
            backend: context.backend,
        }
    }

    fn save(&self) -> Value {
        json!({
            "lines": self.lines.iter().filter_map(|line_data| serde_json::to_value(line_data).ok()).collect::<Vec<Value>>(),
        })
    }
}

impl PlotPanel {
    fn plot(&self, ui: &mut Ui) -> Response {
        let samples = self
            .lines
            .iter()
            .map(|line_data| line_data.samples(self.buffer_history))
            .collect::<Vec<_>>();
        let latest_timestamp = samples
            .iter()
            .flat_map(|samples| samples.iter().map(|(timestamp, _)| *timestamp))
            .max();

        let plot_points = self
            .lines
            .iter()
            .zip(samples.iter())
            .filter(|(line_data, _)| !line_data.is_hidden)
            .map(|(line_data, samples)| {
                (
                    line_data.plot(samples, latest_timestamp),
                    line_data.show_scatter,
                    line_data.is_highlighted,
                    line_data.color,
                )
            })
            .collect::<Vec<_>>();

        EguiPlot::new(ui.id().with("value_plot"))
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                for (plot_points, show_scatter, is_highlighted, color) in &plot_points {
                    if *show_scatter {
                        // TODO(oleflb): use actual name?
                        let points = Points::new("time-series", plot_points.points())
                            .color(*color)
                            .radius(3.0_f32)
                            .shape(MarkerShape::Diamond)
                            .highlight(*is_highlighted);
                        plot_ui.points(points);
                    }

                    let line = Line::new("time-series", plot_points.points())
                        .color(*color)
                        .highlight(*is_highlighted);
                    plot_ui.line(line);
                }
            })
            .response
    }

    fn show_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut history_in_seconds = self.buffer_history.as_secs_f64();
            let widget = DragValue::new(&mut history_in_seconds)
                .range(0.0..=600.0)
                .prefix("History [s]:");
            if ui.add(widget).changed() {
                self.buffer_history = Duration::from_secs_f64(history_in_seconds);
                for buffer in self
                    .lines
                    .iter_mut()
                    .filter_map(|data| data.subscription.as_ref())
                {
                    buffer.set_retention(plot_retention(self.buffer_history));
                }
            }
        });
    }
}

impl Widget for &mut PlotPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let plot_response = self.plot(ui);
        self.show_menu(ui);

        let mut id = 0;
        self.lines.retain_mut(|line_data| {
            ui.horizontal(|ui| {
                let delete_button = Button::new("X");
                let delete_button = ui.add(delete_button);

                let hide_button_face = if line_data.is_hidden { "H" } else { "V" };

                if ui.button(hide_button_face).clicked() {
                    line_data.is_hidden = !line_data.is_hidden;
                }

                ui.checkbox(&mut line_data.show_scatter, "Scatter");

                line_data.show_settings(ui, id, &self.backend, self.buffer_history);
                id += 1;
                !delete_button.clicked()
            })
            .inner
        });

        if ui.button("✚").clicked() {
            self.lines.push(LineData::new(
                DEFAULT_LINE_COLORS
                    .get(self.lines.len())
                    .copied()
                    .unwrap_or(Color32::TRANSPARENT),
            ));
        }

        plot_response
    }
}

#[cfg(test)]
mod tests {
    use mlua::{Function, LuaSerdeExt};
    use serde_json::json;

    use super::*;

    #[test]
    fn invalid_lua_text_sets_error_and_keeps_identity_fallback() {
        let mut line_data = LineData::new(Color32::WHITE);
        line_data.lua_text = "function (".to_string();

        line_data.set_lua();

        assert!(
            line_data
                .lua_error
                .as_deref()
                .is_some_and(|error| !error.is_empty())
        );
        let lua_function: Function = line_data.lua.globals().get("conversion_function").unwrap();
        let value = lua_function
            .call::<f64>(line_data.lua.to_value(&json!(42.0)).unwrap())
            .unwrap();
        assert_eq!(value, 42.0);
    }

    #[test]
    fn latest_only_sampling_uses_latest_sample() {
        let sample = (Time::from_nanos(5), json!(42));

        let samples = samples_for_retention(
            RetentionPolicy::LatestOnly,
            || Some(sample.clone()),
            Vec::new,
        );

        assert_eq!(samples, vec![sample]);
    }

    #[test]
    fn time_window_sampling_uses_window_samples() {
        let sample = (Time::from_nanos(5), json!(42));
        let window_samples = vec![sample.clone()];

        let samples = samples_for_retention(
            RetentionPolicy::time_window(Duration::from_secs(1)).unwrap(),
            || panic!("latest sample should not be read for time-window retention"),
            || window_samples.clone(),
        );

        assert_eq!(samples, vec![sample]);
    }

    #[test]
    fn time_window_sampling_falls_back_to_latest_when_window_samples_are_empty() {
        let sample = (Time::from_nanos(5), json!(42));

        let samples = samples_for_retention(
            RetentionPolicy::time_window(Duration::from_secs(1)).unwrap(),
            || Some(sample.clone()),
            Vec::new,
        );

        assert_eq!(samples, vec![sample]);
    }

    #[test]
    fn time_window_sampling_does_not_duplicate_latest_when_window_samples_exist() {
        let sample = (Time::from_nanos(5), json!(42));
        let window_samples = vec![sample.clone()];

        let samples = samples_for_retention(
            RetentionPolicy::time_window(Duration::from_secs(1)).unwrap(),
            || Some(sample.clone()),
            || window_samples.clone(),
        );

        assert_eq!(samples, vec![sample]);
    }
}
