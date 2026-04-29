use std::{collections::VecDeque, time::Duration};

use chrono::{DateTime, Utc};
use eframe::egui::{Color32, DragValue, Response, Ui, Vec2, Widget};
use egui_plot::{Line, Plot as EguiPlot, PlotImage, PlotPoint, PlotPoints};
use serde_json::Value;
use serde_json::json;

use crate::{
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};

const DEFAULT_HISTORY_SECONDS: f32 = 5.0;
const DEFAULT_TIME_PER_FRAME_SECONDS: f32 = 0.064;

type Frequency = f32;
type Magnitude = f32;
type Spectrum = Vec<(Frequency, Magnitude)>;
type Spectra = Vec<Spectrum>;

pub struct AudioSpectrumPanel {
    buffer: BufferHandle<Option<Spectra>>,
    waterfall_history: VecDeque<Vec<f32>>,
    waterfall_texture: Option<eframe::egui::TextureHandle>,
    max_frequency: f32,
    history_seconds: f32,
    time_per_frame: Duration,
    selected_waterfall_channel: usize,
    y_max_smoothed: f32,
    y_hysteresis_factor: f32,
    current_max_magnitude: f32,
}

impl<'a> Panel<'a> for AudioSpectrumPanel {
    const NAME: &'static str = "Audio Spectrum";

    fn new(context: PanelCreationContext) -> Self {
        let default_path = "Audio.additional_outputs.audio_spectrums".to_string();
        let path = context
            .value
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .map(|s| s.to_string())
            .unwrap_or(default_path.clone());

        let value_buffer = context.robot.subscribe_value(path.clone());

        let history_seconds = context
            .value
            .and_then(|value| value.get("history_seconds"))
            .and_then(|value| value.as_f64())
            .unwrap_or(DEFAULT_HISTORY_SECONDS as f64) as f32;
        let selected_waterfall_channel = context
            .value
            .and_then(|value| value.get("waterfall_channel"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;

        let history_frames =
            (history_seconds / DEFAULT_TIME_PER_FRAME_SECONDS.max(1e-4)).max(1.0) as usize;
        Self {
            buffer: value_buffer,
            waterfall_history: VecDeque::with_capacity(history_frames),
            waterfall_texture: None,
            max_frequency: 8000.0,
            history_seconds,
            time_per_frame: Duration::from_secs_f32(DEFAULT_TIME_PER_FRAME_SECONDS),
            selected_waterfall_channel,
            y_max_smoothed: 0.5,
            y_hysteresis_factor: 0.98,
            current_max_magnitude: 0.001,
        }
    }

    fn save(&self) -> Value {
        json!({
            "history_seconds": self.history_seconds,
            "waterfall_channel": self.selected_waterfall_channel
        })
    }
}

fn magnitude_to_color(magnitude: f32, max_magnitude: f32) -> Color32 {
    let normalized = (magnitude / max_magnitude).clamp(0.0, 1.0);
    let intensity = (normalized * 4.0).min(4.0);

    let (r, g, b) = if intensity < 1.0 {
        (0, 0, (intensity * 255.0) as u8)
    } else if intensity < 2.0 {
        let t = intensity - 1.0;
        (0, (t * 255.0) as u8, 255)
    } else if intensity < 3.0 {
        let t = intensity - 2.0;
        ((t * 255.0) as u8, 255, (255.0 * (1.0 - t)) as u8)
    } else {
        let t = intensity - 3.0;
        (255, (255.0 * (1.0 - t)) as u8, 0)
    };

    Color32::from_rgb(r, g, b)
}

fn draw_color_legend(ui: &mut Ui, max_magnitude: f32) {
    ui.vertical(|ui| {
        ui.label("Magnitude:");
        let legend_width = 200.0;
        let legend_height = 15.0;
        let (rect, _response) = ui.allocate_exact_size(
            Vec2::new(legend_width, legend_height),
            eframe::egui::Sense::hover(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let steps = 50;
            let step_width = legend_width / steps as f32;

            for i in 0..steps {
                let normalized = i as f32 / steps as f32;
                let color = magnitude_to_color(normalized * max_magnitude, max_magnitude);
                let x = rect.min.x + i as f32 * step_width;
                painter.rect_filled(
                    eframe::egui::Rect::from_min_size(
                        eframe::egui::pos2(x, rect.min.y),
                        Vec2::new(step_width + 1.0, legend_height),
                    ),
                    0.0,
                    color,
                );
            }
            painter.rect_stroke(
                rect,
                0.0,
                eframe::egui::Stroke::new(1.0, Color32::GRAY),
                eframe::egui::StrokeKind::Outside,
            );
        }

        let label_height = ui.text_style_height(&eframe::egui::TextStyle::Body);
        let (label_rect, _response) = ui.allocate_exact_size(
            Vec2::new(legend_width, label_height),
            eframe::egui::Sense::hover(),
        );
        if ui.is_rect_visible(label_rect) {
            let painter = ui.painter();
            let font_id = eframe::egui::TextStyle::Body.resolve(ui.style());
            let text_color = ui.visuals().text_color();
            painter.text(
                label_rect.left_top(),
                eframe::egui::Align2::LEFT_TOP,
                "0",
                font_id.clone(),
                text_color,
            );
            painter.text(
                label_rect.right_top(),
                eframe::egui::Align2::RIGHT_TOP,
                format!("{:.3}", max_magnitude),
                font_id,
                text_color,
            );
        }
    });
}

fn duration_to_seconds(duration: Duration) -> f32 {
    duration.as_secs_f32()
}

impl Widget for &mut AudioSpectrumPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if let Ok(Some(timestamp)) = self.buffer.get_last_timestamp() {
                let date: DateTime<Utc> = timestamp.into();
                ui.label(date.format("%T%.3f").to_string());
            }
        });

        let current_spectra: Option<Spectra> = match self.buffer.get_last_value() {
            Ok(Some(value)) => value,
            Ok(None) => return ui.label("no data available"),
            Err(error) => return ui.label(format!("{error:#}")),
        };

        if let Some(ref spectra) = current_spectra {
            if spectra.is_empty() {
                self.waterfall_history.clear();
            } else {
                if self.selected_waterfall_channel >= spectra.len() {
                    self.selected_waterfall_channel = 0;
                    self.waterfall_history.clear();
                }
                let spectrum = &spectra[self.selected_waterfall_channel];
                let magnitudes: Vec<f32> =
                    spectrum.iter().map(|(_, magnitude)| *magnitude).collect();
                if !magnitudes.is_empty() {
                    if let Some((frequency, _)) = spectrum.last() {
                        self.max_frequency = *frequency;
                    }

                    let frame_max = magnitudes.iter().cloned().fold(0.0f32, f32::max);
                    let target_max = (frame_max * 0.5).max(0.001);
                    let current = self.current_max_magnitude.max(0.001);
                    let large_change_ratio = 1.5;

                    if target_max > current * large_change_ratio {
                        self.current_max_magnitude = target_max;
                    } else if target_max * large_change_ratio < current {
                        let ratio = current / target_max;
                        let decay_factor = if ratio > 4.0 {
                            0.5
                        } else if ratio > 2.0 {
                            0.8
                        } else {
                            0.95
                        };
                        self.current_max_magnitude = (current * decay_factor).max(target_max);
                    }

                    self.waterfall_history.push_front(magnitudes);
                    let max_frames = (self.history_seconds
                        / duration_to_seconds(self.time_per_frame).max(1e-4))
                        as usize;
                    while self.waterfall_history.len() > max_frames {
                        self.waterfall_history.pop_back();
                    }
                }
            }
        }

        let current_y_max = current_spectra
            .as_ref()
            .and_then(|spectrum| spectrum.first())
            .map(|spectrum| spectrum.iter().map(|(_, m)| *m).fold(0.0f32, f32::max))
            .unwrap_or(0.1);

        if current_y_max > self.y_max_smoothed {
            self.y_max_smoothed = current_y_max * 1.1;
        } else {
            self.y_max_smoothed = self.y_max_smoothed * self.y_hysteresis_factor
                + current_y_max * (1.0 - self.y_hysteresis_factor);
        }
        self.y_max_smoothed = self.y_max_smoothed.max(0.01); // Minimum y max

        let available_height = ui.available_height() - 60.0;
        let spectrum_height = available_height * 0.35;
        let waterfall_height = available_height * 0.55;

        let link_group = ui.id().with("spectrum_link");

        let plot_response = EguiPlot::new(ui.id().with("audio_spectrum_plot"))
            .legend(Default::default())
            .height(spectrum_height)
            .link_axis(link_group, [true, false])
            .include_y(0.0)
            .include_y(self.y_max_smoothed as f64)
            .auto_bounds([true, false])
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
                if let Some(ref spectra) = current_spectra {
                    for (channel_idx, spectrum) in spectra.iter().enumerate() {
                        if !spectrum.is_empty() {
                            let points: PlotPoints = spectrum
                                .iter()
                                .map(|(freq, mag)| [*freq as f64, *mag as f64])
                                .collect();
                            let line = Line::new(format!("Channel {}", channel_idx), points);
                            plot_ui.line(line);
                        }
                    }
                }
            });

        ui.horizontal(|ui| {
            draw_color_legend(ui, self.current_max_magnitude);
            ui.separator();
            ui.label("History:");
            let mut history_secs = self.history_seconds;
            if ui
                .add(
                    DragValue::new(&mut history_secs)
                        .range(1.0..=30.0)
                        .suffix(" s")
                        .speed(0.1),
                )
                .changed()
            {
                self.history_seconds = history_secs;
                let max_frames = (self.history_seconds
                    / duration_to_seconds(self.time_per_frame).max(1e-4))
                    as usize;
                while self.waterfall_history.len() > max_frames {
                    self.waterfall_history.pop_back();
                }
            }

            ui.separator();
            ui.label("Waterfall channel:");
            let available_channels = current_spectra.as_ref().map_or(0, |spectra| spectra.len());
            if available_channels > 0 {
                let previous_channel = self.selected_waterfall_channel;
                eframe::egui::ComboBox::from_id_salt(ui.id().with("waterfall_channel"))
                    .selected_text(format!("Channel {}", self.selected_waterfall_channel))
                    .show_ui(ui, |ui| {
                        for channel_idx in 0..available_channels {
                            ui.selectable_value(
                                &mut self.selected_waterfall_channel,
                                channel_idx,
                                format!("Channel {}", channel_idx),
                            );
                        }
                    });
                if self.selected_waterfall_channel != previous_channel {
                    self.waterfall_history.clear();
                }
            } else {
                ui.label("n/a");
            }
        });

        ui.label("Waterfall (Time vs Frequency)");

        let waterfall_response = if !self.waterfall_history.is_empty() {
            let number_frequencies = self.waterfall_history.front().map(|v| v.len()).unwrap_or(0);
            let number_times = self.waterfall_history.len();

            if number_frequencies > 0 && number_times > 0 {
                let max_magnitude = self.current_max_magnitude.max(0.001);

                let mut pixels = Vec::with_capacity(number_times * number_frequencies);
                for row in self.waterfall_history.iter() {
                    for &magnitude in row.iter() {
                        pixels.push(magnitude_to_color(magnitude, max_magnitude));
                    }
                }

                let image = eframe::egui::ColorImage::from_rgba_unmultiplied(
                    [number_frequencies, number_times],
                    &pixels
                        .iter()
                        .flat_map(|color| [color.r(), color.g(), color.b(), color.a()])
                        .collect::<Vec<_>>(),
                );

                self.waterfall_texture = Some(ui.ctx().load_texture(
                    "waterfall",
                    image,
                    eframe::egui::TextureOptions::NEAREST,
                ));

                let time_per_frame = duration_to_seconds(self.time_per_frame).max(1e-4);
                let total_time = number_times as f64 * time_per_frame as f64;
                let major = 1.0;
                let medium = 1.0;
                let minor = 0.1;

                EguiPlot::new(ui.id().with("waterfall_plot"))
                    .height(waterfall_height)
                    .link_axis(link_group, [true, false])
                    .allow_drag(true)
                    .allow_zoom(true)
                    .show_axes([true, true])
                    .show_grid([true, true])
                    .x_grid_spacer(egui_plot::log_grid_spacer(10))
                    .y_grid_spacer(egui_plot::uniform_grid_spacer(move |_| {
                        [major, medium, minor]
                    }))
                    .y_axis_formatter(move |mark, _| format!("{:.1}s", mark.value))
                    .label_formatter(move |_, value| {
                        format!("Freq: {:.0} Hz\nTime: {:.2}s ago", value.x, value.y)
                    })
                    .show(ui, |plot_ui| {
                        if let Some(texture) = &self.waterfall_texture {
                            let image = PlotImage::new(
                                "waterfall",
                                texture.id(),
                                PlotPoint::new(self.max_frequency as f64 / 2.0, total_time / 2.0),
                                [self.max_frequency, total_time as f32],
                            );
                            plot_ui.image(image);
                        }
                    })
                    .response
            } else {
                ui.label("Waiting for data...")
            }
        } else {
            ui.label("Waiting for data...")
        };

        plot_response.response | waterfall_response
    }
}
