use eframe::egui::{InnerResponse, Response, ScrollArea, TextEdit, Ui, Widget};
use log::{error, info};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    nao::Nao, panel::Panel, repository_configuration_handler::RepositoryConfigurationHandler,
    value_buffer::ValueBuffer,
};

use super::parameter::{add_save_button, subscribe};

struct CameraParameterSubscriptions {
    path: String,
    value_buffer: ValueBuffer,
    value: String,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    repository_configuration_handler: RepositoryConfigurationHandler,
    extrinsic_rotation_subscriptions: [CameraParameterSubscriptions; 2],
}

const CAMERA_KEY_BASE: &str = "camera_matrix_parameters.vision_";
const ROTATIONS: &str = ".extrinsic_rotations";

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Manual Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let extrinsic_rotation_subscriptions = ["top", "bottom"].map(|name| {
            let path = CAMERA_KEY_BASE.to_owned() + name + ROTATIONS;

            let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
            let value_buffer = subscribe(nao.clone(), &path, update_notify_sender.clone());

            info!("Subscribing to path {}", path);

            CameraParameterSubscriptions {
                path,
                value_buffer: value_buffer.unwrap(),
                value: String::new(),
                update_notify_receiver,
            }
        });

        let connection_url = nao.get_address();
        let repository_configuration_handler = RepositoryConfigurationHandler::new();
        repository_configuration_handler.print_nao_ids(connection_url.clone());

        Self {
            nao,
            repository_configuration_handler,
            extrinsic_rotation_subscriptions,
        }
    }
}

fn add_calibration_ui_components_for_one_camera(
    ui: &mut Ui,
    camera_index: usize,
    panel: &mut ManualCalibrationPanel,
) -> InnerResponse<()> {
    let nao = Arc::clone(&panel.nao);
    let extrinsic_rotation_subscriptions =
        &mut panel.extrinsic_rotation_subscriptions[camera_index];
    let rotations_parameter_buffer = &extrinsic_rotation_subscriptions.value_buffer;
    let rotation_value = &mut extrinsic_rotation_subscriptions.value;
    let rotations_path = &extrinsic_rotation_subscriptions.path;
    let rotations_update_notify_receiver =
        &mut extrinsic_rotation_subscriptions.update_notify_receiver;
    let repository_configuration_handler = &panel.repository_configuration_handler;

    ui.horizontal(|ui| {
        ui.label(rotations_path);
        let settable = !rotation_value.is_empty();
        ui.add_enabled_ui(settable, |ui| {
            if ui.button(format!("Set {}", camera_index)).clicked() {
                match serde_json::value::to_value(&rotation_value) {
                    Ok(value) => {
                        panel.nao.update_parameter_value(rotations_path, value);
                    }
                    Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                }
            }
        });

        add_save_button(
            ui,
            rotations_path,
            rotation_value,
            nao,
            repository_configuration_handler,
            settable,
        );

        match rotations_parameter_buffer.get_latest() {
            Ok(value) => {
                if rotations_update_notify_receiver.try_recv().is_ok() {
                    *rotation_value = serde_json::to_string_pretty(&value).unwrap();
                }
                ScrollArea::vertical()
                    .id_source(rotations_path)
                    .show(ui, |ui: &mut Ui| {
                        ui.add(
                            TextEdit::multiline(rotation_value)
                                .code_editor()
                                .desired_width(f32::INFINITY),
                        )
                    });
            }
            Err(error) => {
                ui.label(format!("{error:#?}"));
            }
        }
    })
}

impl Widget for &mut ManualCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            add_calibration_ui_components_for_one_camera(ui, 0, self);
            add_calibration_ui_components_for_one_camera(ui, 1, self);
        })
        .response
    }
}
