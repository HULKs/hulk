use std::sync::Arc;

use chrono::{DateTime, Utc};
use eframe::egui::{Response, Ui, Widget};
use egui_plot::{Line, Plot as EguiPlot, PlotPoints};
use hulk_widgets::{PathFilter, RobotPathCompletionEdit};
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    value_buffer::BufferHandle,
};

pub struct AudioSpectrumPanel {
    robot: Arc<Robot>,
    path: String,
    buffer: Option<BufferHandle<Value>>,
}

impl<'a> Panel<'a> for AudioSpectrumPanel {
    const NAME: &'static str = "Audio Spectrum";

    fn new(context: PanelCreationContext) -> Self {
        let path = match context.value.and_then(|value| value.get("path")) {
            Some(Value::String(string)) => string.to_string(),
            _ => "audio.additional_outputs.audio_spectrums".to_string(),
        };
        let buffer = if !path.is_empty() {
            Some(context.robot.subscribe_json(path.clone()))
        } else {
            None
        };
        Self {
            robot: context.robot,
            path,
            buffer,
        }
    }

    fn save(&self) -> Value {
        json!({
            "path": self.path.clone()
        })
    }
}

impl Widget for &mut AudioSpectrumPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let edit_response = ui
            .horizontal(|ui| {
                let edit_response = ui.add(RobotPathCompletionEdit::new(
                    ui.id().with("audio-spectrum-panel"),
                    self.robot.latest_paths(),
                    &mut self.path,
                    PathFilter::Readable,
                ));
                if edit_response.changed() {
                    self.buffer = Some(self.robot.subscribe_json(self.path.clone()));
                }
                if let Some(buffer) = &self.buffer {
                    if let Ok(Some(timestamp)) = buffer.get_last_timestamp() {
                        let date: DateTime<Utc> = timestamp.into();
                        ui.label(date.format("%T%.3f").to_string());
                    }
                }
                edit_response
            })
            .inner;

        let plot_response = EguiPlot::new(ui.id().with("audio_spectrum_plot"))
            .legend(Default::default())
            .label_formatter(|name, value| {
                if !name.is_empty() {
                    format!("{}\nFreq: {:.0} Hz\nMag: {:.4}", name, value.x, value.y)
                } else {
                    format!("Freq: {:.0} Hz\nMag: {:.4}", value.x, value.y)
                }
            })
            .allow_drag(true)
            .allow_zoom(true)
            .allow_scroll(true)
            .show_axes([true, true])
            .show_grid([true, true])
            .x_grid_spacer(egui_plot::log_grid_spacer(10))
            .show(ui, |plot_ui| {
                if let Some(buffer) = &self.buffer {
                    match buffer.get_last() {
                        Ok(Some(datum)) => {
                            // The data is Vec<Vec<(f32, f32)>> where outer vec is channels,
                            // inner vec is frequency bins with (frequency, magnitude) pairs
                            match serde_json::from_value::<Vec<Vec<(f32, f32)>>>(
                                datum.value.clone(),
                            ) {
                                Ok(spectrums) => {
                                    if spectrums.is_empty() {
                                        // Data exists but is empty
                                    } else {
                                        for (channel_idx, spectrum) in spectrums.iter().enumerate()
                                        {
                                            if !spectrum.is_empty() {
                                                let points: PlotPoints = spectrum
                                                    .iter()
                                                    .map(|(freq, mag)| [*freq as f64, *mag as f64])
                                                    .collect();

                                                let line = Line::new(
                                                    format!("Channel {}", channel_idx),
                                                    points,
                                                );

                                                plot_ui.line(line);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Show error in the plot area
                                    eprintln!("Failed to parse spectrum data: {}", e);
                                }
                            }
                        }
                        Ok(None) => {
                            // No data available yet
                        }
                        Err(e) => {
                            eprintln!("Error getting buffer data: {}", e);
                        }
                    }
                } else {
                    // No buffer - show message
                }
            });

        edit_response | plot_response.response
    }
}
