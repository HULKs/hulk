use std::{str::FromStr, sync::Arc};

use communication::client::CyclerOutput;
use eframe::egui::{Response, Slider, Ui, Widget};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct BehaviorSimulatorPanel {
    nao: Arc<Nao>,
    chosen_time: usize,
    value_buffer: ValueBuffer,
    frame_count: ValueBuffer,
    update_notify_receiver: mpsc::Receiver<()>,
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
            chosen_time: 0,
            value_buffer,
            frame_count,
            update_notify_receiver,
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
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
                if ui
                    .add_sized(
                        ui.available_size(),
                        Slider::new(
                            &mut self.chosen_time,
                            0..=self
                                .frame_count
                                .get_latest()
                                .ok()
                                .and_then(|v| v.as_u64())
                                .unwrap_or(1) as usize
                                - 1,
                        )
                        .text("Time"),
                    )
                    .changed()
                {
                    self.nao
                        .update_parameter_value("time", self.chosen_time.into());
                };
            });
        })
        .response
    }
}
