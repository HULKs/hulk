use std::sync::Arc;

use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};
use eframe::egui::{Response, Ui, Widget};

use nalgebra::{vector, Vector2};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct LookAtLitePanel {
    nao: Arc<Nao>,
    value_buffer: Option<ValueBuffer>,
    head_angles: Option<Vector2<f32>>,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub fn subscribe(
    nao: Arc<Nao>,
    path: &str,
    update_notify_sender: mpsc::Sender<()>,
) -> Option<ValueBuffer> {
    if path.is_empty() {
        return None;
    }

    let value_buffer = nao.subscribe_parameter(path);
    value_buffer.listen_to_updates(update_notify_sender);
    Some(value_buffer)
}

const PENALIZED_POSE_HEAD_PATH: &'static str = "penalized_pose.head";

impl Panel for LookAtLitePanel {
    const NAME: &'static str = "Look-At Lite";

    fn new(nao: Arc<Nao>, _: Option<&Value>) -> Self {
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        let value_buffer = subscribe(
            nao.clone(),
            PENALIZED_POSE_HEAD_PATH,
            update_notify_sender.clone(),
        );

        Self {
            nao,
            value_buffer,
            head_angles: None,
            update_notify_receiver,
        }
    }
}

impl Widget for &mut LookAtLitePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let look_down_head_angles: Vector2<f32> = vector![0.0, 0.0];
            let look_up_head_angles: Vector2<f32> = vector![-10.0, 0.0];
            let current_angles = self.head_angles.clone();
            if ui.button("Look Up").clicked() {
                self.head_angles = Some(look_up_head_angles);
            }
            if ui.button("Look Down").clicked() {
                self.head_angles = Some(look_down_head_angles);
            }

            if current_angles != self.head_angles {
                if let Some(head_angles) = self.head_angles {
                    self.nao.update_parameter_value(
                        PENALIZED_POSE_HEAD_PATH,
                        json!({"pitch": head_angles[0],"yaw": head_angles[1]}),
                    );
                }
            }

            if let Some(buffer) = &self.value_buffer {
                match buffer.get_latest() {
                    Ok(value) => {
                        if self.update_notify_receiver.try_recv().is_ok() {
                            let value_to_f32 = |v: &Value| v.as_f64().and_then(|v| Some(v as f32));
                            if let (Some(pitch), Some(yaw)) = (
                                value.get("pitch").and_then(value_to_f32),
                                value.get("yaw").and_then(value_to_f32),
                            ) {
                                self.head_angles = Some(vector![pitch, yaw]);
                            }
                        }
                    }
                    Err(error) => {
                        ui.label(error);
                    }
                }
            }
        })
        .response
    }
}
