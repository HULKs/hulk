use std::{str::FromStr, sync::Arc};

use eframe::{
    egui::{
        self,
        plot::{Line, PlotPoints},
        widgets::plot::Plot as EguiPlot,
        DragValue, Response, Widget,
    },
    epaint::Color32,
    Storage,
};
use log::{error, info};

use communication::CyclerOutput;

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::ValueBuffer};

struct LineData {
    output_key: String,
    value_buffer: Option<ValueBuffer>,
    color: Color32,
}

impl Default for LineData {
    fn default() -> Self {
        Self {
            output_key: String::new(),
            value_buffer: None,
            color: Color32::TRANSPARENT,
        }
    }
}

pub struct PlotPanel {
    line_datas: Vec<LineData>,
    buffer_size: usize,
    nao: Arc<Nao>,
}

impl Panel for PlotPanel {
    const NAME: &'static str = "Plot";

    fn new(nao: Arc<Nao>, _storage: Option<&dyn Storage>) -> Self {
        Self {
            nao,
            line_datas: Vec::new(),
            buffer_size: 1_000,
        }
    }
}

impl Widget for &mut PlotPanel {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let lines: Vec<_> = self
            .line_datas
            .iter()
            .map(|data| {
                let values = if let Some(buffer) = &data.value_buffer {
                    match buffer.get_buffered() {
                        Ok(buffered_values) => {
                            ui.ctx().request_repaint();
                            PlotPoints::from_iter(buffered_values.iter().rev().enumerate().map(
                                |(i, value)| {
                                    let value = match value {
                                        serde_json::Value::Bool(value) => *value as u8 as f64,
                                        serde_json::Value::Number(number) => {
                                            number.as_f64().unwrap()
                                        }
                                        _ => f64::NAN,
                                    };
                                    [i as f64, value]
                                },
                            ))
                        }
                        _ => PlotPoints::default(),
                    }
                } else {
                    PlotPoints::default()
                };
                Line::new(values).color(data.color)
            })
            .collect();
        let plot = EguiPlot::new("value_plot")
            .view_aspect(2.0)
            .show(ui, |plot_ui| {
                for line in lines {
                    plot_ui.line(line);
                }
            });
        ui.horizontal(|ui| {
            if ui
                .add(
                    DragValue::new(&mut self.buffer_size)
                        .clamp_range(0..=10_000)
                        .prefix("Buffer Size:"),
                )
                .changed()
            {
                for line_data in &self.line_datas {
                    if let Some(buffer) = &line_data.value_buffer {
                        buffer.set_buffer_size(self.buffer_size);
                    }
                }
            }
            if ui.button("Add").clicked() {
                self.line_datas.push(LineData::default());
            }
        });
        for line_data in self.line_datas.iter_mut() {
            ui.horizontal(|ui| {
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
                ui.color_edit_button_srgba(&mut line_data.color)
            });
        }
        plot.response
    }
}
