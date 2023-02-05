use std::{str::FromStr, sync::Arc};

use communication::client::CyclerOutput;
use eframe::egui::{Response, Slider, Ui, Widget};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct BehaviorSimulatorPanel {
    nao: Arc<Nao>,
    update_notify_receiver: mpsc::Receiver<()>,

    chosen_time: usize,
    playing: bool,

    value_buffer: ValueBuffer,
    frame_count: ValueBuffer,
}

impl Panel for BehaviorSimulatorPanel {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let value_buffer = nao.subscribe_parameter("time");
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        value_buffer.listen_to_updates(update_notify_sender);

        let frame_count = nao.subscribe_output(
            CyclerOutput::from_str("BehaviorSimulator.main_outputs.frame_count").unwrap(),
        );
        Self {
            nao,
            update_notify_receiver,

            chosen_time: 0,
            playing: false,

            value_buffer,
            frame_count,
        }
    }
}

impl Widget for &mut BehaviorSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        while self.update_notify_receiver.try_recv().is_ok() {
            if let Ok(value) = self.value_buffer.require_latest() {
                self.chosen_time = value;
            }
        }
        let mut new_time = None;
        let response = ui
            .vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                    let mut time = self.chosen_time;
                    if ui
                        .add_sized(
                            ui.available_size(),
                            Slider::new(
                                &mut time,
                                0..=self
                                    .frame_count
                                    .get_latest()
                                    .ok()
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1) as usize
                                    - 1,
                            )
                            .smart_aim(false)
                            .text("Time"),
                        )
                        .changed()
                    {
                        new_time = Some(time);
                    };
                });
                ui.checkbox(&mut self.playing, "Play");
            })
            .response;

        if self.playing || ui.button(">>").clicked() {
            new_time = Some(new_time.unwrap_or(self.chosen_time) + 100);
            self.nao
                .update_parameter_value("time", self.chosen_time.into());
        }
        if let Some(new_time) = new_time {
            self.chosen_time = new_time % self.frame_count.require_latest().unwrap_or(1);
            self.nao
                .update_parameter_value("time", self.chosen_time.into());
        }
        response
    }
}
