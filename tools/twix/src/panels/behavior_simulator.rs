use std::sync::Arc;

use eframe::egui::{self, Response, ScrollArea, Slider, TextEdit, Ui, Widget};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::{completion_edit::CompletionEdit, nao::Nao, panel::Panel, value_buffer::ValueBuffer};

pub struct BehaviorSimulatorPanel {
    nao: Arc<Nao>,
    chosen_time: usize,
    value_buffer: ValueBuffer,
    update_notify_sender: mpsc::Sender<()>,
    update_notify_receiver: mpsc::Receiver<()>,
}

impl Panel for BehaviorSimulatorPanel {
    const NAME: &'static str = "Behavior Simulator";

    fn new(nao: Arc<Nao>, value: Option<&Value>) -> Self {
        let value_buffer = nao.subscribe_parameter("time");
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        value_buffer.listen_to_updates(update_notify_sender.clone());

        Self {
            nao,
            chosen_time: 0,
            value_buffer,
            update_notify_sender,
            update_notify_receiver,
        }
    }
}

impl Widget for &mut BehaviorSimulatorPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .add(Slider::new(&mut self.chosen_time, 0..=999).text("Time"))
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
