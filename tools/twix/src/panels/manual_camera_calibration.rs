use color_eyre::eyre::Context;
use eframe::egui::{Response, Slider, Ui, Widget};
use log::{error, info};
use serde_json::Value;
use std::{ops::RangeInclusive, sync::Arc};
use tokio::sync::mpsc;
use types::configuration::CameraMatrixParameters;

use crate::{
    nao::Nao, panel::Panel, repository_parameters::RepositoryParameters, value_buffer::ValueBuffer,
};

use super::parameter::{add_save_button, subscribe};

struct CameraParameterSubscriptions<DeserializedValueType> {
    human_friendly_label: String,
    path: String,
    value_buffer: Option<ValueBuffer>,
    value: DeserializedValueType,
    update_notify_receiver: mpsc::Receiver<()>,
}

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    repository_parameters: Option<RepositoryParameters>,
    extrinsic_rotation_subscriptions:
        [CameraParameterSubscriptions<Option<CameraMatrixParameters>>; 2],
}

const CAMERA_KEY_BASE: &str = "camera_matrix_parameters.vision_";

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Manual Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let extrinsic_rotation_subscriptions = ["Top", "Bottom"].map(|name| {
            let path = CAMERA_KEY_BASE.to_owned() + name.to_lowercase().as_str();

            let (update_notify_sender, update_notify_receiver) = mpsc::channel(1);
            let value_buffer = subscribe(nao.clone(), &path, update_notify_sender);

            info!("Subscribing to path {}", path);

            CameraParameterSubscriptions {
                human_friendly_label: name.to_string(),
                path,
                value_buffer,
                value: None,
                update_notify_receiver,
            }
        });

        Self {
            nao,
            repository_parameters: RepositoryParameters::try_default().ok(),
            extrinsic_rotation_subscriptions,
        }
    }
}

fn add_extrinsic_calibration_ui_components(
    ui: &mut Ui,
    nao: Arc<Nao>,
    repository_parameters: &Option<RepositoryParameters>,
    camera_matrix_parameters_subscription: &mut CameraParameterSubscriptions<
        Option<CameraMatrixParameters>,
    >,
) {
    let camera_parameter_buffer_option = &camera_matrix_parameters_subscription.value_buffer;
    let mut camera_parameter_option = &mut camera_matrix_parameters_subscription.value;
    let label = &camera_matrix_parameters_subscription.human_friendly_label;
    let camera_matrix_subscription_path = &camera_matrix_parameters_subscription.path;
    let rotations_update_notify_receiver =
        &mut camera_matrix_parameters_subscription.update_notify_receiver;

    let slider_minimum_decimals = 2;
    let slider_maximum_decimals = 6;
    let extrinsic_maximum_degrees = 15.0;

    ui.horizontal(|ui| {
        if let Some(buffer) = &camera_parameter_buffer_option {
            match buffer.get_latest() {
                Ok(value) => {
                    if rotations_update_notify_receiver.try_recv().is_ok() {
                        *camera_parameter_option =
                            serde_json::from_value::<CameraMatrixParameters>(value).ok();
                    }
                }
                Err(error) => {
                    ui.label(format!("{error:#?}"));
                }
            }
        }

        ui.label(format!("{label:#} Camera"));

        add_save_button(
            ui,
            camera_matrix_subscription_path,
            || {
                serde_json::to_value(&camera_parameter_option)
                    .wrap_err("Conveting CameraMatrixParameters to serde_json::Value failed.")
            },
            nao.clone(),
            repository_parameters,
        );
    });

    ui.style_mut().spacing.slider_width = ui.available_size().x - 100.0;
    let mut changed = false;
    ui.collapsing(
        format!("Intrinsic Parameters {label:#}"),
        |ui| match &mut camera_parameter_option {
            Some(camera_parameter_value) => {
                ui.label("Focal Lengths (Normalized)");
                for (axis_value, axis_name) in camera_parameter_value
                    .focal_lengths
                    .iter_mut()
                    .zip(["X", "Y"])
                {
                    let slider = Slider::new(axis_value, RangeInclusive::new(0.0, 2.0))
                        .text(axis_name)
                        .smart_aim(false)
                        .min_decimals(slider_minimum_decimals)
                        .max_decimals(slider_maximum_decimals);
                    if ui.add(slider).changed() {
                        changed = true
                    };
                }
                ui.label("Optical Centre (Normalized)");
                for (axis_value, axis_name) in camera_parameter_value
                    .cc_optical_center
                    .iter_mut()
                    .zip(["X", "Y"])
                {
                    let slider = Slider::new(axis_value, RangeInclusive::new(0.0, 1.0))
                        .text(axis_name)
                        .smart_aim(false)
                        .min_decimals(slider_minimum_decimals)
                        .max_decimals(slider_maximum_decimals);
                    if ui.add(slider).changed() {
                        changed = true
                    };
                }
            }
            _ => {
                ui.label("Intrinsic parameters not recieved.");
            }
        },
    );

    ui.label(format!(
        "Extrinsic Rotations [{}°, {}°]",
        -extrinsic_maximum_degrees, extrinsic_maximum_degrees
    ));
    match &mut camera_parameter_option {
        Some(camera_parameter_value) => {
            for (axis_value, axis_name) in camera_parameter_value
                .extrinsic_rotations
                .iter_mut()
                .zip(["Roll", "Pitch", "Yaw"])
            {
                let slider = Slider::new(
                    axis_value,
                    RangeInclusive::new(-extrinsic_maximum_degrees, extrinsic_maximum_degrees),
                )
                .text(axis_name)
                .smart_aim(false);
                if ui.add(slider).changed() {
                    changed = true
                };
            }
        }
        _ => {
            ui.label("Extrinsic parameters not recieved.");
        }
    };
    if changed {
        if let Some(camera_parameter_value) = camera_parameter_option {
            match serde_json::value::to_value(camera_parameter_value) {
                Ok(value) => {
                    nao.update_parameter_value(camera_matrix_subscription_path, value);
                }
                Err(error) => error!("Failed to serialize parameter value: {error:#?}"),
            }
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
                    &self.repository_parameters,
                    extrinsic_rotation_subscription,
                );

                ui.separator();
            }
        })
        .response
    }
}
