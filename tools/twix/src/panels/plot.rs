use std::{str::FromStr, sync::Arc};

use eframe::{
    egui::{
        plot::{Line, PlotPoints},
        widgets::plot::Plot as EguiPlot,
        CollapsingHeader, DragValue, Response, TextEdit, TextStyle, Ui, Widget,
    },
    epaint::Color32,
};
use log::{error, info};

use color_eyre::eyre::{eyre, Result, WrapErr};
use communication::client::CyclerOutput;
use mlua::{Function, Lua, LuaSerdeExt};
use serde_json::{to_string_pretty, Value};

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

struct LineData {
    output_key: String,
    value_buffer: Option<ValueBuffer>,
    color: Color32,
    lua: Lua,
    lua_text: String,
    lua_error: Option<String>,
}

impl LineData {
    fn new(color: Color32) -> Self {
        let lua = Lua::new();
        let lua_text = "function (value)\n  return value\nend".to_string();
        lua.globals()
            .set(
                "conversion_function",
                lua.load(&lua_text).eval::<Function>().unwrap(),
            )
            .unwrap();
        Self {
            output_key: String::new(),
            value_buffer: None,
            color,
            lua,
            lua_text,
            lua_error: None,
        }
    }

    fn plot(&self) -> Line {
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
                                [i as f64, value]
                            },
                        ))
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        Line::new(values).color(self.color)
    }
}

pub struct PlotPanel {
    line_datas: Vec<LineData>,
    buffer_size: usize,
    nao: Arc<Nao>,
}

impl Panel for PlotPanel {
    const NAME: &'static str = "Plot";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        Self {
            nao,
            line_datas: Vec::new(),
            buffer_size: 1_000,
        }
    }
}

impl PlotPanel {
    fn plot(&self, ui: &mut Ui) -> Response {
        EguiPlot::new(ui.id().with("value_plot"))
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                for line in self.line_datas.iter().map(|entry| entry.plot()) {
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
            if ui.button("Add").clicked() {
                self.line_datas.push(LineData::new(
                    DEFAULT_LINE_COLORS
                        .get(self.line_datas.len())
                        .copied()
                        .unwrap_or(Color32::TRANSPARENT),
                ));
            }
        });
    }
}

impl Widget for &mut PlotPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let plot_response = self.plot(ui);
        self.show_menu(ui);
        for (i, line_data) in self.line_datas.iter_mut().enumerate() {
            ui.horizontal_top(|ui| {
                let subscription_field = ui.add(CompletionEdit::outputs(
                    &mut line_data.output_key,
                    self.nao.as_ref(),
                ));
                if subscription_field.changed() {
                    info!("Subscribing: {}", line_data.output_key);
                    line_data.value_buffer = match CyclerOutput::from_str(&line_data.output_key) {
                        Ok(output) => {
                            let buffer = self.nao.subscribe_output(output);
                            buffer.set_buffer_size(self.buffer_size);
                            Some(buffer)
                        }
                        Err(error) => {
                            error!("Failed to subscribe: {:#}", error);
                            None
                        }
                    };
                }
                ui.color_edit_button_srgba(&mut line_data.color);
                let id_source = ui.id().with("conversion_collapse").with(i);
                CollapsingHeader::new("Conversion Function")
                    .id_source(id_source)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let latest_value = get_latest_value(&line_data.value_buffer);
                            let content = latest_value
                                .and_then(|value| {
                                    to_string_pretty(&value).wrap_err("failed to prettify value")
                                })
                                .unwrap_or_else(|error| error.to_string());
                            ui.label(content);
                            let code_edit = TextEdit::multiline(&mut line_data.lua_text)
                                .font(TextStyle::Monospace)
                                .code_editor()
                                .lock_focus(true);
                            if ui.add(code_edit).changed() {
                                line_data.lua_error = match line_data
                                    .lua
                                    .load(&line_data.lua_text)
                                    .eval::<Function>()
                                {
                                    Ok(function) => {
                                        line_data
                                            .lua
                                            .globals()
                                            .set("conversion_function", function)
                                            .unwrap();
                                        None
                                    }
                                    Err(error) => Some(format!("{error:#}")),
                                };
                            }
                            if let Some(error) = &line_data.lua_error {
                                ui.colored_label(Color32::RED, error);
                            } else if let Ok(value) = get_latest_value(&line_data.value_buffer) {
                                let lua_function: Function =
                                    line_data.lua.globals().get("conversion_function").unwrap();
                                let value =
                                    lua_function.call::<_, f64>(line_data.lua.to_value(&value));
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
