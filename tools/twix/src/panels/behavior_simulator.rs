use std::sync::Arc;

use communication::messages::TextOrBinary;
use eframe::egui::{Color32, Response, Slider, Ui, Widget};
use serde_json::{json, Value};

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct BehaviorSimulatorPanel {
    nao: Arc<Nao>,

    selected_frame: usize,
    selected_robot: usize,
    playing: bool,
    playing_start: f64,

    selected_frame_updater: BufferHandle<usize>,
    frame_count: BufferHandle<usize>,
}

impl Panel for BehaviorSimulatorPanel {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let selected_frame_updater = nao.subscribe_value("parameters.selected_frame");

        let frame_count = nao.subscribe_value("BehaviorSimulator.main_outputs.frame_count");
        let selected_frame = value
            .and_then(|value| value.get("selected_frame"))
            .and_then(|value| value.as_bool())
            .unwrap_or_default() as usize;
        let selected_robot = value
            .and_then(|value| value.get("selected_robot"))
            .and_then(|value| value.as_u64())
            .unwrap_or_default() as usize;
        let playing = value
            .and_then(|value| value.get("playing"))
            .and_then(|value| value.as_bool())
            .unwrap_or_default();
        let playing_start = value
            .and_then(|value| value.get("playing_start"))
            .and_then(|value| value.as_f64())
            .unwrap_or_default();
        Self {
            nao,

            selected_frame,
            selected_robot,
            playing,
            playing_start,

            selected_frame_updater,
            frame_count,
        }
    }

    fn save(&self) -> Value {
        json!({
            "selected_frame": self.selected_frame.clone(),
            "selected_robot": self.selected_robot.clone(),
            "playing": self.playing.clone(),
            "playing_start": self.playing_start.clone()
        })
    }
}

impl Widget for &mut BehaviorSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.selected_frame_updater.has_changed() {
            self.selected_frame_updater.mark_as_seen();
            if let Some(selected_frame) =
                self.selected_frame_updater.get_last_value().ok().flatten()
            {
                self.selected_frame = selected_frame
            }
        }
        let frame_count = match self.frame_count.get_last_value() {
            Ok(Some(frame_count)) => frame_count,
            Ok(None) => return ui.label("no frame data yet"),
            Err(error) => return ui.colored_label(Color32::RED, format!("Error: {error}")),
        };
        let mut new_frame = None;
        let response = ui
            .vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                    let mut frame = self.selected_frame;
                    if ui
                        .add_sized(
                            ui.available_size(),
                            Slider::new(&mut frame, 0..=frame_count - 1)
                                .smart_aim(false)
                                .text("Frame"),
                        )
                        .changed()
                    {
                        new_frame = Some(frame);
                    };
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            ui.available_size(),
                            Slider::new(&mut self.selected_robot, 1..=7)
                                .smart_aim(false)
                                .text("Robot"),
                        )
                        .changed()
                    {
                        self.nao.write(
                            "parameters.selected_robot",
                            TextOrBinary::Text(self.selected_robot.into()),
                        );
                    };
                });
                if ui.checkbox(&mut self.playing, "Play").changed() || new_frame.is_some() {
                    self.playing_start = ui.input(|input| input.time)
                        - new_frame.unwrap_or(self.selected_frame) as f64 / 83.0;
                };
            })
            .response;

        if self.playing {
            let now = ui.input(|input| input.time);
            let time_elapsed = now - self.playing_start;
            new_frame = Some((time_elapsed * 83.0 * 5.0) as usize);
        }
        if ui.button(">>").clicked() {
            new_frame = Some(new_frame.unwrap_or(self.selected_frame) + 10);
        }
        if let Some(new_frame) = new_frame {
            self.selected_frame = new_frame % frame_count;
            self.nao.write(
                "parameters.selected_frame",
                TextOrBinary::Text(self.selected_frame.into()),
            );
        }
        response
    }
}
