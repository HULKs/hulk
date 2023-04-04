use std::{str::FromStr, sync::Arc};

use eframe::{
    egui::{
        plot::{Line, PlotPoints},
        widgets::plot::Plot as EguiPlot,
        Button, CollapsingHeader, DragValue, Response, RichText, TextEdit, TextStyle, Ui, Widget,
    },
    epaint::Color32,
};
use log::{error, info};

use color_eyre::eyre::{eyre, Result, WrapErr};
use communication::client::CyclerOutput;
use mlua::{Function, Lua, LuaSerdeExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty, Value};

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::ValueBuffer};

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
    output_key: String,
    #[serde(skip)]
    value_buffer: Option<ValueBuffer>,
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
    hidden: bool,
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
            output_key: String::new(),
            value_buffer: None,
            color,
            lua,
            lua_text,
            lua_error: None,
            is_highlighted: false,
            hidden: false,
        };

        line_data.set_lua();
        line_data
    }

    fn set_highlighted(&mut self, is_highlighted: bool) {
        self.is_highlighted = is_highlighted
    }

    fn plot(&self, buffer_size: usize) -> Line {
        let lua_function: Function = self.lua.globals().get("conversion_function").unwrap();
        let values = self
            .value_buffer
            .as_ref()
            .map(|buffer| {
                buffer
                    .get_buffered()
                    .map(|buffered_values| {
                        PlotPoints::from_iter(buffered_values.iter().rev().enumerate().map(
                            |(i, value)| {
                                let value = lua_function
                                    .call::<_, f64>(self.lua.to_value(value))
                                    .unwrap_or(f64::NAN);
                                [(buffer_size + i - buffered_values.len()) as f64, value]
                            },
                        ))
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        Line::new(values)
            .color(self.color)
            .highlight(self.is_highlighted)
    }

    fn draw_row(&mut self, ui: &mut Ui, nao: Arc<Nao>, buffer_size: usize, id: usize) {
        ui.horizontal_top(|ui| {
            let subscription_field =
                ui.add(CompletionEdit::outputs(&mut self.output_key, nao.as_ref()));
            self.set_highlighted(subscription_field.hovered());
            if subscription_field.changed() {
                info!("Subscribing: {}", self.output_key);
                self.subscribe_key(nao.clone(), buffer_size);
            }
            ui.color_edit_button_srgba(&mut self.color);
            let id_source = ui.id().with("conversion_collapse").with(id);
            CollapsingHeader::new("Conversion Function")
                .id_source(id_source)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let latest_value = get_latest_value(&self.value_buffer);
                        let content = latest_value
                            .and_then(|value| {
                                to_string_pretty(&value).wrap_err("failed to prettify value")
                            })
                            .unwrap_or_else(|error| error.to_string());
                        ui.label(content);
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
                        } else if let Ok(value) = get_latest_value(&self.value_buffer) {
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

    fn subscribe_key(&mut self, nao: Arc<Nao>, buffer_size: usize) {
        self.value_buffer = match CyclerOutput::from_str(&self.output_key) {
            Ok(output) => {
                let buffer = nao.subscribe_output(output);
                buffer.set_buffer_size(buffer_size);
                Some(buffer)
            }
            Err(error) => {
                error!("Failed to subscribe: {:#}", error);
                None
            }
        };
    }
}

pub struct PlotPanel {
    line_datas: Vec<LineData>,
    buffer_size: usize,
    nao: Arc<Nao>,
}

impl Panel for PlotPanel {
    const NAME: &'static str = "Plot";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        const DEFAULT_BUFFER_SIZE: usize = 1_000;

        let line_datas = if let Some(line_datas) =
            value.and_then(|value| value["subscribe_keys"].as_array())
        {
            line_datas
                .iter()
                .filter_map(|line_data| {
                    if let Ok(mut line_data) = serde_json::from_value::<LineData>(line_data.clone())
                    {
                        line_data.set_lua();
                        line_data.subscribe_key(nao.clone(), 1000);
                        Some(line_data)
                    } else {
                        None
                    }
                })
                .collect::<Vec<LineData>>()
        } else {
            vec![]
        };

        PlotPanel {
            line_datas,
            buffer_size: DEFAULT_BUFFER_SIZE,
            nao,
        }
    }

    fn save(&self) -> Value {
        json!({
            "subscribe_keys": self.line_datas.iter().filter_map(|line_data| serde_json::to_value(line_data).ok()).collect::<Vec<Value>>(),
        })
    }
}

impl PlotPanel {
    fn plot(&self, ui: &mut Ui) -> Response {
        let maximum_buffer_size = self
            .line_datas
            .iter()
            .filter_map(|line_data| {
                line_data
                    .value_buffer
                    .as_ref()
                    .and_then(|buffer| buffer.get_buffer_size().ok())
            })
            .max()
            .unwrap_or(self.buffer_size);

        EguiPlot::new(ui.id().with("value_plot"))
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                for line in self
                    .line_datas
                    .iter()
                    .filter(|line_data| !line_data.hidden)
                    .map(|entry| entry.plot(maximum_buffer_size))
                {
                    plot_ui.line(line);
                }
            })
            .response
    }

    fn show_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.buffer_size)
                        .clamp_range(0..=10_000)
                        .prefix("Buffer Size:"),
                )
                .changed()
            {
                for buffer in self
                    .line_datas
                    .iter_mut()
                    .filter_map(|data| data.value_buffer.as_ref())
                {
                    buffer.set_buffer_size(self.buffer_size);
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
        self.line_datas.retain_mut(|line_data| {
            ui.horizontal(|ui| {
                let delete_button = Button::new(RichText::new("❌").color(Color32::WHITE).strong())
                    .fill(Color32::RED);
                let delete_button = ui.add(delete_button);

                let hide_button = Button::new(
                    RichText::new(if line_data.hidden { "V" } else { "H" })
                        .color(Color32::WHITE)
                        .strong(),
                )
                .fill(Color32::GRAY);
                if ui.add(hide_button).clicked() {
                    line_data.hidden = !line_data.hidden;
                }

                line_data.draw_row(ui, self.nao.clone(), self.buffer_size, id);
                id += 1;
                !delete_button.clicked()
            })
            .inner
        });

        if ui.button("✚").clicked() {
            self.line_datas.push(LineData::new(
                DEFAULT_LINE_COLORS
                    .get(self.line_datas.len())
                    .copied()
                    .unwrap_or(Color32::TRANSPARENT),
            ));
        }

        plot_response
    }
}

fn get_latest_value(value_buffer: &Option<ValueBuffer>) -> Result<Value> {
    let buffer = value_buffer
        .as_ref()
        .ok_or(eyre!("nothing subscribed yet"))?;
    buffer
        .get_latest()
        .map_err(|error| eyre!("failed to get latest value: {error}"))
}
