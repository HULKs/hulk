use crate::{nao::Nao, panel::Panel, value_buffer::ValueBuffer};
use eframe::egui::{Response, ScrollArea, Slider, Ui, Widget};
use log::info;
use nalgebra::{point, Point2};
use serde_json::Value;
use std::{ops::RangeInclusive, sync::Arc};
use tokio::sync::mpsc;
use types::{CameraPosition, FieldDimensions, HeadMotion, MotionCommand};

#[derive(PartialEq)]
enum LookAtType {
    PenaltyBoxFromCenter,
    Manual,
}

pub struct LookAtLitePanel {
    nao: Arc<Nao>,
    camera_position: Option<CameraPosition>,
    look_at_target: Point2<f32>,
    look_at_mode: LookAtType,
    is_enabled: bool,
    field_dimensions: Option<ValueBuffer>,
    field_dimensions_update_notify_receiver: mpsc::Receiver<()>,
    motion_command: Option<ValueBuffer>,
    motion_command_update_notify_receiver: mpsc::Receiver<()>,
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

const INJECTED_MOTION_COMMAND: &'static str = "behavior.injected_motion_command";
const DEFAULT_TARGET: Point2<f32> = point![1.0, 0.0];

impl Panel for LookAtLitePanel {
    const NAME: &'static str = "Look-At Lite";

    fn new(nao: Arc<Nao>, _: Option<&Value>) -> Self {
        let (update_notify_sender, field_dimensions_update_notify_receiver) = mpsc::channel(1);
        let field_dimensions = subscribe(nao.clone(), "field_dimensions", update_notify_sender);

        let (update_notify_sender, motion_command_update_notify_receiver) = mpsc::channel(1);
        let motion_command = subscribe(
            nao.clone(),
            "control.main.motion_command",
            update_notify_sender,
        );

        Self {
            nao,
            camera_position: Some(CameraPosition::Top),
            look_at_target: DEFAULT_TARGET,
            look_at_mode: LookAtType::PenaltyBoxFromCenter,
            is_enabled: false,
            field_dimensions,
            field_dimensions_update_notify_receiver,
            motion_command,
            motion_command_update_notify_receiver,
        }
    }
}

impl Widget for &mut LookAtLitePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
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

            ui.label("Select look-at mode");
            ui.radio_value(
                &mut self.look_at_mode,
                LookAtType::PenaltyBoxFromCenter,
                "Look at penalty box from center circle",
            );
            ui.radio_value(
                &mut self.look_at_mode,
                LookAtType::Manual,
                "Manual target (Robot Coordinates)",
            );

            let current_field_dimensions = self.field_dimensions.as_ref().and_then(|buffer| {
                buffer.get_latest().ok().and_then(|latest| {
                    if self
                        .field_dimensions_update_notify_receiver
                        .try_recv()
                        .is_ok()
                    {
                        serde_json::from_value::<FieldDimensions>(latest).ok()
                    } else {
                        None
                    }
                })
            });

            self.look_at_target = match self.look_at_mode {
                LookAtType::PenaltyBoxFromCenter => {
                    if let Some(dimensions) = current_field_dimensions {
                        let half_field_length = dimensions.length / 2.0;
                        point![half_field_length, 0.0]
                    } else {
                        DEFAULT_TARGET
                    }
                }
                LookAtType::Manual => {
                    let max_dimension = current_field_dimensions
                        .map_or(10.0, |dimensions: FieldDimensions| dimensions.length);

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

            if self.is_enabled && ui.button("Send Command").clicked() {
                send_standing_look_at(self.nao.as_ref(), self.look_at_target, self.camera_position);
            };

            if let Some(buffer) = &self.motion_command {
                match buffer.get_latest() {
                    Ok(value) => {
                        let mut current_motion_command_string = if self
                            .motion_command_update_notify_receiver
                            .try_recv()
                            .is_ok()
                        {
                            serde_json::to_string_pretty(&value).unwrap()
                        } else {
                            "".to_string()
                        };

                        ScrollArea::vertical().show(ui, |ui| {
                            ui.code_editor(&mut current_motion_command_string);
                        });
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
    info!("Setting motion command: {motion_command:#?}");
    nao.update_parameter_value(
        INJECTED_MOTION_COMMAND,
        serde_json::to_value(motion_command).unwrap(),
    );
}
