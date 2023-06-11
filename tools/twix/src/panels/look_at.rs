use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};
use communication::client::CyclerOutput;
use eframe::{
    egui::{Response, Slider, TextFormat, Ui, Widget},
    epaint::{text::LayoutJob, Color32},
};
use nalgebra::{point, Point2};
use serde_json::Value;
use std::{ops::RangeInclusive, str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use types::{CameraPosition, FieldDimensions, HeadMotion, MotionCommand};

use super::parameter::subscribe;

#[derive(PartialEq)]
enum LookAtType {
    PenaltyBoxFromCenter,
    Manual,
}

pub struct LookAtPanel {
    nao: Arc<Nao>,
    camera_position: Option<CameraPosition>,
    look_at_target: Point2<f32>,
    look_at_mode: LookAtType,
    is_enabled: bool,
    field_dimensions: Option<FieldDimensions>,
    field_dimensions_buffer: ValueBuffer,
    field_dimensions_update_notify_receiver: mpsc::Receiver<()>,
    motion_command: ValueBuffer,
}

const INJECTED_MOTION_COMMAND: &str = "behavior.injected_motion_command";
const DEFAULT_TARGET: Point2<f32> = point![1.0, 0.0];
const FALLBACK_MAX_FIELD_DIMENSION: f32 = 10.0;

impl Panel for LookAtPanel {
    const NAME: &'static str = "Look At";

    fn new(nao: Arc<Nao>, _: Option<&Value>) -> Self {
        let (field_dimensions_update_notify_sender, field_dimensions_update_notify_receiver) =
            mpsc::channel(1);
        let field_dimensions_buffer = subscribe(
            nao.clone(),
            "field_dimensions",
            field_dimensions_update_notify_sender,
        )
        .expect("Failed to subscribe to field_dimensions");
        let motion_command = nao.subscribe_output(
            CyclerOutput::from_str("Control.main_outputs.motion_command")
                .expect("Failed to subscribe to main_outputs.motion_command"),
        );

        Self {
            nao,
            camera_position: Some(CameraPosition::Top),
            look_at_target: DEFAULT_TARGET,
            look_at_mode: LookAtType::PenaltyBoxFromCenter,
            is_enabled: false,
            field_dimensions: None,
            field_dimensions_buffer,
            field_dimensions_update_notify_receiver,
            motion_command,
        }
    }
}

impl Widget for &mut LookAtPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let error_format = TextFormat {
                color: Color32::RED,
                ..Default::default()
            };
            let mut status_text_job = LayoutJob::default();

            let current_motion_command = match self.motion_command.get_latest() {
                Ok(value) => serde_json::from_value(value).ok(),
                Err(error) => {
                    status_text_job.append(&error, 0.0, error_format.clone());
                    None
                }
            };

            let is_safe_to_override_current_motion_command =
                current_motion_command.as_ref().map_or(false, |command| {
                    matches!(
                        command,
                        MotionCommand::Penalized
                            | MotionCommand::Stand { .. }
                            | MotionCommand::Walk { .. }
                            | MotionCommand::InWalkKick { .. }
                    )
                });
            if !is_safe_to_override_current_motion_command {
                status_text_job.append(
                    "Cannot safely override motion, please put the NAO into a standing position!",
                    1.0,
                    error_format.clone(),
                );
            }

            self.is_enabled = self.is_enabled && is_safe_to_override_current_motion_command;

            ui.add_enabled_ui(is_safe_to_override_current_motion_command, |ui| {
                if ui
                    .checkbox(&mut self.is_enabled, "Enable Motion Override")
                    .changed()
                {
                    if self.is_enabled {
                        send_standing_look_at(
                            self.nao.as_ref(),
                            self.look_at_target,
                            self.camera_position,
                        );
                    } else {
                        self.nao
                            .update_parameter_value(INJECTED_MOTION_COMMAND, Value::Null);
                    }
                }
            });

            if let Ok(latest) = self.field_dimensions_buffer.get_latest() {
                if self
                    .field_dimensions_update_notify_receiver
                    .try_recv()
                    .is_ok()
                {
                    self.field_dimensions = serde_json::from_value::<FieldDimensions>(latest).ok();
                }
            }
            if self.field_dimensions.is_none() {
                status_text_job.append(
                    " Missing field dimensions data, some modes are disabled.",
                    1.0,
                    error_format,
                );
            }

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Select look-at mode.");

                    ui.add_enabled_ui(self.field_dimensions.is_some(), |ui| {
                        ui.radio_value(
                            &mut self.look_at_mode,
                            LookAtType::PenaltyBoxFromCenter,
                            "Look at penalty box from center circle",
                        )
                    });

                    ui.radio_value(
                        &mut self.look_at_mode,
                        LookAtType::Manual,
                        "Manual target (Robot Coordinates)",
                    );
                });
                ui.vertical(|ui| {
                    ui.label("Camera to look at with:");
                    ui.radio_value(
                        &mut self.camera_position,
                        Some(CameraPosition::Top),
                        "Top Camera",
                    );
                    ui.radio_value(
                        &mut self.camera_position,
                        Some(CameraPosition::Bottom),
                        "Bottom Camera",
                    );
                    ui.radio_value(&mut self.camera_position, None, "Automatic");
                });
            });

            self.look_at_target = match self.look_at_mode {
                LookAtType::PenaltyBoxFromCenter => {
                    if let Some(dimensions) = &self.field_dimensions {
                        let half_field_length = dimensions.length / 2.0;
                        point![half_field_length, 0.0]
                    } else {
                        DEFAULT_TARGET
                    }
                }
                LookAtType::Manual => {
                    let max_dimension = self
                        .field_dimensions
                        .as_ref()
                        .map_or(FALLBACK_MAX_FIELD_DIMENSION, |dimensions| dimensions.length);

                    ui.add(
                        Slider::new(
                            &mut self.look_at_target.x,
                            RangeInclusive::new(-max_dimension, max_dimension),
                        )
                        .text("x")
                        .smart_aim(false),
                    );
                    ui.add(
                        Slider::new(
                            &mut self.look_at_target.y,
                            RangeInclusive::new(-max_dimension, max_dimension),
                        )
                        .text("y")
                        .smart_aim(false),
                    );

                    self.look_at_target
                }
            };

            ui.add_enabled_ui(self.is_enabled, |ui| {
                if ui.button("Send Command").clicked() {
                    send_standing_look_at(
                        self.nao.as_ref(),
                        self.look_at_target,
                        self.camera_position,
                    );
                }
            });

            ui.label(status_text_job);
        })
        .response
    }
}

fn send_standing_look_at(
    nao: &Nao,
    look_at_target: Point2<f32>,
    camera_option: Option<CameraPosition>,
) {
    let motion_command = Some(MotionCommand::Stand {
        head: HeadMotion::LookAt {
            target: look_at_target,
            camera: camera_option,
        },
        is_energy_saving: false,
    });
    nao.update_parameter_value(
        INJECTED_MOTION_COMMAND,
        serde_json::to_value(motion_command).unwrap(),
    );
}
