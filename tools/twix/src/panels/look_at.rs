use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};
use eframe::{
    egui::{Response, Slider, TextFormat, Ui, Widget},
    epaint::{text::LayoutJob, Color32, FontId},
};
use nalgebra::{point, Point2};
use serde_json::Value;
use std::{ops::RangeInclusive, sync::Arc};
use types::{
    camera_position::CameraPosition,
    field_dimensions::FieldDimensions,
    motion_command::{HeadMotion, MotionCommand},
};

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
    field_dimensions_buffer: ValueBuffer,
    motion_command_buffer: ValueBuffer,
}

const INJECTED_MOTION_COMMAND: &str = "behavior.injected_motion_command";
const DEFAULT_TARGET: Point2<f32> = point![1.0, 0.0];
const FALLBACK_MAX_FIELD_DIMENSION: f32 = 10.0;

impl Panel for LookAtPanel {
    const NAME: &'static str = "Look At";

    fn new(nao: Arc<Nao>, _: Option<&Value>) -> Self {
        let field_dimensions_buffer = nao.subscribe_parameter("field_dimensions");
        let motion_command_buffer = nao.subscribe_output("Control.main_outputs.motion_command");

        Self {
            nao,
            camera_position: Some(CameraPosition::Top),
            look_at_target: DEFAULT_TARGET,
            look_at_mode: LookAtType::PenaltyBoxFromCenter,
            is_enabled: false,
            field_dimensions_buffer,
            motion_command_buffer,
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
            let leading_space = 10.0f32;

            let current_motion_command: Option<MotionCommand> = match self
                .motion_command_buffer
                .parse_latest()
            {
                Ok(value) => {
                    status_text_job.append(
                        format!("Current Motion: {:?}.", value).as_str(),
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(14.0),
                            ..Default::default()
                        },
                    );
                    Some(value)
                }
                Err(error) => {
                    status_text_job.append(error.to_string().as_str(), 0.0, error_format.clone());
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
                    leading_space,
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

            let current_field_dimensions: Option<FieldDimensions> =
                match self.field_dimensions_buffer.parse_latest() {
                    Ok(value) => Some(value),
                    Err(error) => {
                        status_text_job.append(
                            format!("Field dimensions are not available: {}", error).as_str(),
                            leading_space,
                            error_format,
                        );
                        None
                    }
                };

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Select look-at mode.");

                    ui.add_enabled_ui(current_field_dimensions.is_some(), |ui| {
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
                    current_field_dimensions
                        .as_ref()
                        .map_or(DEFAULT_TARGET, |dimensions| {
                            let half_field_length = dimensions.length / 2.0;
                            point![half_field_length, 0.0]
                        })
                }
                LookAtType::Manual => {
                    let max_dimension = current_field_dimensions
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
    });
    nao.update_parameter_value(
        INJECTED_MOTION_COMMAND,
        serde_json::to_value(motion_command).unwrap(),
    );
}
