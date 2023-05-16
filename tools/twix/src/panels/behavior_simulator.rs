use std::{str::FromStr, sync::Arc};

use communication::client::CyclerOutput;
use eframe::egui::{Response, Slider, Ui, Widget};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct BehaviorSimulatorPanel {
    nao: Arc<Nao>,
    update_notify_receiver: mpsc::Receiver<()>,

    selected_frame: usize,
    selected_robot: usize,
    playing: bool,
    playing_start: f64,

    value_buffer: ValueBuffer,
    frame_count: ValueBuffer,
}

impl Panel for BehaviorSimulatorPanel {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let value_buffer = nao.subscribe_parameter("selected_frame");
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        value_buffer.listen_to_updates(update_notify_sender);

        let frame_count = nao.subscribe_output(
            CyclerOutput::from_str("BehaviorSimulator.main_outputs.frame_count").unwrap(),
        );
        Self {
            nao,
            update_notify_receiver,

            selected_frame: 0,
            selected_robot: 0,
            playing: false,
            playing_start: 0.0,

            value_buffer,
            frame_count,
        }
    }
}

impl Widget for &mut BehaviorSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        while self.update_notify_receiver.try_recv().is_ok() {
            if let Ok(value) = self.value_buffer.require_latest() {
                self.selected_frame = value;
            }
        }
        let mut new_frame = None;
        let response = ui
            .vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                    let mut frame = self.selected_frame;
                    if ui
                        .add_sized(
                            ui.available_size(),
                            Slider::new(
                                &mut frame,
                                0..=self
                                    .frame_count
                                    .get_latest()
                                    .ok()
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize
                                    - 1,
                            )
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
                        self.nao
                            .update_parameter_value("selected_robot", self.selected_robot.into());
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
            new_frame = Some((time_elapsed * 83.0) as usize);
        }
        if ui.button(">>").clicked() {
            new_frame = Some(new_frame.unwrap_or(self.selected_frame) + 10);
        }
        if let Some(new_frame) = new_frame {
            self.selected_frame = new_frame % self.frame_count.require_latest().unwrap_or(1);
            self.nao
                .update_parameter_value("selected_frame", self.selected_frame.into());
        }
        response
    }
}
