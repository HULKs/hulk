use std::{f32::consts::FRAC_PI_2, str::FromStr, sync::Arc};

use eframe::{
    egui::{ComboBox, Response, Ui, Widget},
    epaint::{Color32, Pos2, Stroke},
    Storage,
};
use log::info;
use nalgebra::{point, vector, Point2, Similarity2, Vector2};
use serde_json::Value;
use tokio::sync::mpsc;
use types::{CameraPosition, HeadMotion, MotionCommand};

use crate::{nao::Nao, panel::Panel, twix_painter::TwixPainter, value_buffer::ValueBuffer};

use super::parameter::subscribe;

pub struct LookAtPanel {
    nao: Arc<Nao>,
    camera_position: CameraPosition,
    motion_command: ValueBuffer,
    is_enabled: bool,
}

impl Panel for LookAtPanel {
    const NAME: &'static str = "Look At";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
        let motion_command = subscribe(
            nao.clone(),
            "control.main.motion_command",
            update_notify_sender,
        )
        .unwrap();

        Self {
            nao,
            camera_position: CameraPosition::Top,
            is_enabled: false,
            motion_command,
        }
    }
}

impl Widget for &mut LookAtPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ComboBox::from_label("Camera")
            .selected_text(format!("{:?}", self.camera_position))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.camera_position, CameraPosition::Top, "Top");
                ui.selectable_value(&mut self.camera_position, CameraPosition::Bottom, "Bottom");
            });
        if ui
            .checkbox(&mut self.is_enabled, "Enable Motion Override")
            .changed()
        {
            if self.is_enabled {
                send_standing_look_at(self.nao.as_ref(), point![1.0, 0.0], self.camera_position);
            } else {
                self.nao.update_parameter_value(
                    "control.behavior.injected_motion_command",
                    Value::Null,
                );
            }
        }
        let (painter_response, painter) = TwixPainter::allocate_new(ui);
        //     ui,
        //     vector![3.0, 3.0],
        //     Similarity2::identity(),
        //     Similarity2::new(vector![1.5, 1.5], -FRAC_PI_2, 1.0),
        //     1.0,
        // );
        painter.rect_filled(point![1.5, -1.5], point![-1.5, 1.5], Color32::DARK_GREEN);
        painter.line_segment(
            point![1.5, 0.0],
            point![-1.5, 0.0],
            Stroke::new(0.1, Color32::BLACK),
        );
        painter.line_segment(
            point![0.0, 1.5],
            point![0.0, -1.5],
            Stroke::new(0.1, Color32::BLACK),
        );
        if let Some(position) = painter_response.interact_pointer_pos() {
            if self.is_enabled {
                let look_at_target = painter.transform_pixel_to_world(position);
                send_standing_look_at(self.nao.as_ref(), look_at_target, self.camera_position);
            }
        }
        if let Ok(value) = self.motion_command.get_latest() {
            let motion_command: MotionCommand = serde_json::from_value(value).unwrap();
            if let MotionCommand::SitDown {
                head: HeadMotion::LookAt { target, .. },
            }
            | MotionCommand::Stand {
                head: HeadMotion::LookAt { target, .. },
                ..
            }
            | MotionCommand::Walk {
                head: HeadMotion::LookAt { target, .. },
                ..
            }
            | MotionCommand::InWalkKick {
                head: HeadMotion::LookAt { target, .. },
                ..
            } = motion_command
            {
                painter.circle(target, 0.1, Color32::BLUE, Stroke::default())
            }
        }

        painter_response
    }
}

fn send_standing_look_at(nao: &Nao, look_at_target: Point2<f32>, with_camera: CameraPosition) {
    let motion_command = Some(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target: look_at_target,
            camera: Some(with_camera),
        },
        is_energy_saving: false,
    });
    info!("Setting motion command: {motion_command:#?}");
    nao.update_parameter_value(
        "control.behavior.injected_motion_command",
        serde_json::to_value(motion_command).unwrap(),
    );
}
