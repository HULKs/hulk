use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use color_eyre::eyre::{Context, OptionExt};
use eframe::{
    egui::{Button, CollapsingHeader, DragValue, Response, TextEdit, TextStyle, Ui, Widget},
    epaint::Color32,
};
use egui_plot::{Line, Plot as EguiPlot, PlotPoints};
use itertools::Itertools;
use mlua::{Function, Lua, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty, Value};

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::BufferHandle};

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

#[derive(Serialize, Deserialize)]
struct LineData {
    path: String,
    #[serde(skip)]
    buffer: Option<BufferHandle<Value>>,
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
}

impl LineData {
    fn create_lua() -> Lua {
        Lua::new()
    }

    fn set_lua(&mut self) {
        self.lua
            .globals()
            .set(
                "conversion_function",
                self.lua.load(&self.lua_text).eval::<Function>().unwrap(),
            )
            .unwrap();
    }

    fn new(color: Color32) -> Self {
        let lua = LineData::create_lua();
        let lua_text = "function (value)\n  return value\nend".to_string();

        let mut line_data = Self {
            path: String::new(),
            buffer: None,
            color,
            lua,
            lua_text,
            lua_error: None,
            is_highlighted: false,
            is_hidden: false,
        };

        line_data.set_lua();
        line_data
    }

    fn set_highlighted(&mut self, is_highlighted: bool) {
        self.is_highlighted = is_highlighted
    }

    fn plot(&self, latest_timestamp: Option<SystemTime>) -> Line {
        let lua_function: Function = self.lua.globals().get("conversion_function").unwrap();
        let values = self
            .buffer
            .as_ref()
            .map(|buffer| {
                buffer
                    .get()
                    .map(|buffered_values| {
                        PlotPoints::from_iter(buffered_values.iter().map(|datum| {
                            let value = lua_function
                                .call::<_, f64>(self.lua.to_value(&datum.value))
                                .unwrap_or(f64::NAN);
                            [
                                -latest_timestamp
                                    .unwrap()
                                    .duration_since(datum.timestamp)
                                    .unwrap_or(Duration::ZERO)
                                    .as_secs_f64(),
                                value,
                            ]
                        }))
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        Line::new(values)
            .color(self.color)
            .highlight(self.is_highlighted)
    }

    fn show_settings(&mut self, ui: &mut Ui, id: usize, nao: &Nao, buffer_history: Duration) {
        ui.horizontal_top(|ui| {
            let subscription_field = ui.add(CompletionEdit::readable_paths(&mut self.path, nao));
            self.set_highlighted(subscription_field.hovered());
            if subscription_field.changed() {
                let handle = nao.subscribe_buffered_json(&self.path, buffer_history);
                self.buffer = Some(handle);
            }

            ui.color_edit_button_srgba(&mut self.color);

            let id_source = ui.id().with("conversion_collapse").with(id);
            CollapsingHeader::new("Conversion Function")
                .id_source(id_source)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let latest_value = self
                            .buffer
                            .as_ref()
                            .ok_or_eyre("no subscription yet")
                            .and_then(|buffer| buffer.get_last_value())
                            .and_then(|maybe_value| maybe_value.ok_or_eyre("no value yet"));

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
                            self.lua_error = match self.lua.load(&self.lua_text).eval::<Function>()
                            {
                                Ok(function) => {
                                    self.lua
                                        .globals()
                                        .set("conversion_function", function)
                                        .unwrap();
                                    None
                                }
                                Err(error) => Some(format!("{error:#}")),
                            };
                        }
                        if let Some(error) = &self.lua_error {
                            ui.colored_label(Color32::RED, error);
                        } else if let Ok(value) = &latest_value {
                            let lua_function: Function =
                                self.lua.globals().get("conversion_function").unwrap();
                            let value = lua_function.call::<_, f64>(self.lua.to_value(&value));
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
    nao: Arc<Nao>,
}

impl Panel for PlotPanel {
    const NAME: &'static str = "Plot";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        const DEFAULT_BUFFER_HISTORY: Duration = Duration::from_secs(10);

        let lines = value
            .and_then(|value| value["lines"].as_array())
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(|line_data| {
                        let mut line_data =
                            serde_json::from_value::<LineData>(line_data.clone()).ok()?;
                        line_data.set_lua();
                        if !line_data.path.is_empty() {
                            let handle = nao
                                .subscribe_buffered_json(&line_data.path, DEFAULT_BUFFER_HISTORY);
                            line_data.buffer = Some(handle);
                        }
                        Some(line_data)
                    })
                    .collect_vec()
            })
            .unwrap_or_default();

        PlotPanel {
            lines,
            buffer_history: DEFAULT_BUFFER_HISTORY,
            nao,
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
        let latest_timestamp = self
            .lines
            .iter()
            .filter_map(|line_data| {
                let buffer = line_data.buffer.as_ref()?;
                let last = buffer.get_last_timestamp().ok().flatten()?;
                Some(last)
            })
            .max();

        EguiPlot::new(ui.id().with("value_plot"))
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                for line in self
                    .lines
                    .iter()
                    .filter(|line_data| !line_data.is_hidden)
                    .map(|entry| entry.plot(latest_timestamp))
                {
                    plot_ui.line(line);
                }
            })
            .response
    }

    fn show_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut history_in_seconds = self.buffer_history.as_secs_f64();
            let widget = DragValue::new(&mut history_in_seconds)
                .clamp_range(0.0..=600.0)
                .prefix("History [s]:");
            if ui.add(widget).changed() {
                self.buffer_history = Duration::from_secs_f64(history_in_seconds);
                for buffer in self
                    .lines
                    .iter_mut()
                    .filter_map(|data| data.buffer.as_ref())
                {
                    buffer.set_history(self.buffer_history);
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

                line_data.show_settings(ui, id, &self.nao, self.buffer_history);
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
