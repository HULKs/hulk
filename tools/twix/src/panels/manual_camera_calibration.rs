use eframe::egui::{Response, Slider, Ui, Widget};
use log::{error, info};
use nalgebra::Vector3;
use serde_json::Value;
use std::{ops::RangeInclusive, sync::Arc};
use tokio::sync::mpsc;

use crate::{
    nao::Nao, panel::Panel, repository_configuration_handler::RepositoryConfigurationHandler,
    value_buffer::ValueBuffer,
};

use super::parameter::{add_save_button, subscribe};

struct CameraParameterSubscriptions<DeserializedValueType> {
    human_friendly_label: String,
    path: String,
    value_buffer: ValueBuffer,
    value: DeserializedValueType,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    repository_configuration_handler: RepositoryConfigurationHandler,
    extrinsic_rotation_subscriptions: [CameraParameterSubscriptions<Vector3<f32>>; 2],
}

const CAMERA_KEY_BASE: &str = "camera_matrix_parameters.vision_";
const ROTATIONS: &str = ".extrinsic_rotations";

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Manual Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let extrinsic_rotation_subscriptions = ["Top", "Bottom"].map(|name| {
            let path = CAMERA_KEY_BASE.to_owned() + name.to_lowercase().as_str() + ROTATIONS;

            let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
            let value_buffer = subscribe(nao.clone(), &path, update_notify_sender);

            info!("Subscribing to path {}", path);

            CameraParameterSubscriptions {
                human_friendly_label: name.to_string() + " Extrinsic Rotations",
                path,
                value_buffer: value_buffer.unwrap(),
                value: Vector3::zeros(),
                update_notify_receiver,
            }
        });

        let connection_url = nao.get_address();
        let repository_configuration_handler = RepositoryConfigurationHandler::new();
        repository_configuration_handler.print_nao_ids(connection_url);

        Self {
            nao,
            repository_configuration_handler,
            extrinsic_rotation_subscriptions,
        }
    }
}

fn add_extrinsic_calibration_ui_components(
    ui: &mut Ui,
    nao: Arc<Nao>,
    repository_configuration_handler: &RepositoryConfigurationHandler,
    extrinsic_rotation_subscriptions: &mut CameraParameterSubscriptions<Vector3<f32>>,
) {
    let rotations_parameter_buffer = &extrinsic_rotation_subscriptions.value_buffer;
    let rotation_value = &mut extrinsic_rotation_subscriptions.value;
    let label = &extrinsic_rotation_subscriptions.human_friendly_label;
    let rotations_path = &extrinsic_rotation_subscriptions.path;
    let rotations_update_notify_receiver =
        &mut extrinsic_rotation_subscriptions.update_notify_receiver;

    ui.horizontal(|ui| {
        ui.label(label);
        let settable = !rotation_value.is_empty();
        ui.add_enabled_ui(settable, |ui| {
            if ui.button("Set").clicked() {
                match serde_json::value::to_value(&rotation_value) {
                    Ok(value) => {
                        nao.update_parameter_value(rotations_path, value);
                    }
                    Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
                }
            }
        });

        // TODO Fix this as it looks utterly ridiculous.
        match serde_json::value::to_value(&rotation_value) {
            Ok(value) => {
                add_save_button(
                    ui,
                    rotations_path,
                    serde_json::to_string::<Value>(&value).unwrap().as_str(),
                    nao,
                    repository_configuration_handler,
                    settable,
                );
            }
            Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
        }
    });
    match rotations_parameter_buffer.get_latest() {
        Ok(value) => {
            if rotations_update_notify_receiver.try_recv().is_ok() {
                *rotation_value = serde_json::from_value::<Vector3<f32>>(value).unwrap();
            }
            ui.vertical(|ui| {
                for (axis_value, axis_name) in
                    rotation_value.iter_mut().zip(["Roll", "Pitch", "Yaw"])
                {
                    ui.add(
                        Slider::new(axis_value, RangeInclusive::new(-40.0, 40.0))
                            .text(axis_name)
                            .step_by(0.1),
                    );
                }
            });
        }
        Err(error) => {
            ui.label(format!("{error:#?}"));
        }
    }
}

impl Widget for &mut ManualCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            for extrinsic_rotation_subscription in &mut self.extrinsic_rotation_subscriptions {
                add_extrinsic_calibration_ui_components(
                    ui,
                    self.nao.clone(),
                    &self.repository_configuration_handler,
                    extrinsic_rotation_subscription,
                );
            }
        })
        .response
    }
}
