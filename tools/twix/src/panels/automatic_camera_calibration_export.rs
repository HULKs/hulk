use std::sync::Arc;

use eframe::egui::{Response, Ui, Widget};
use log::error;
use nalgebra::Vector3;
use serde_json::Value;

use calibration::corrections::Corrections;
use communication::messages::TextOrBinary;
use parameters::directory::Scope;
use types::primary_state::PrimaryState;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub const TOP_CAMERA_EXTRINSICS_PATH: &str =
    "camera_matrix_parameters.vision_top.extrinsic_rotations";
pub const BOTTOM_CAMERA_EXTRINSICS_PATH: &str =
    "camera_matrix_parameters.vision_bottom.extrinsic_rotations";
pub const ROBOT_BODY_ROTATION_PATH: &str = "camera_matrix_parameters.robot_rotation";
pub struct AutomaticCameraCalibrationExportPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<Vector3<f32>>,
    bottom_camera: BufferHandle<Vector3<f32>>,
    robot_body_rotations: BufferHandle<Vector3<f32>>,
    calibration_corrections: BufferHandle<Value>,
    calibration_measurements: BufferHandle<Value>,
    primary_state: BufferHandle<PrimaryState>,
}

impl Panel for AutomaticCameraCalibrationExportPanel {
    const NAME: &'static str = "Automatic Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let top_camera = nao.subscribe_value(format!("parameters.{TOP_CAMERA_EXTRINSICS_PATH}"));
        let bottom_camera =
            nao.subscribe_value(format!("parameters.{BOTTOM_CAMERA_EXTRINSICS_PATH}"));
        let body_rotations = nao.subscribe_value(format!("parameters.{ROBOT_BODY_ROTATION_PATH}"));
        let calibration_corrections = nao
            .subscribe_json("Control.additional_outputs.calibration_controller.last_corrections");
        let calibration_measurements = nao
            .subscribe_json("Control.additional_outputs.calibration_controller.last_measurements");
        let primary_state = nao.subscribe_value("Control.main_outputs.primary_state");

        Self {
            nao,
            top_camera,
            bottom_camera,
            robot_body_rotations: body_rotations,
            calibration_corrections,
            calibration_measurements,
            primary_state,
        }
    }
}

impl Widget for &mut AutomaticCameraCalibrationExportPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        ui.vertical(|ui| {
            if let Some(value) = self
                .calibration_corrections
                .get_last_value()
                .ok()
                .flatten()
                .and_then(|value| serde_json::from_value::<Corrections>(value).ok())
            {
                let top_angles = value.correction_in_camera_top.clone().euler_angles();
                let bottom_angles = value.correction_in_camera_bottom.euler_angles();
                let body_angles = value.correction_in_robot.euler_angles();

                draw_group(ui, "Top", top_angles, &self.nao, TOP_CAMERA_EXTRINSICS_PATH);
                draw_angles_from_buffer(ui, &self.top_camera);
                ui.separator();

                draw_group(
                    ui,
                    "Bottom",
                    bottom_angles,
                    &self.nao,
                    BOTTOM_CAMERA_EXTRINSICS_PATH,
                );
                draw_angles_from_buffer(ui, &self.bottom_camera);
                ui.separator();

                draw_group(ui, "Body", body_angles, &self.nao, ROBOT_BODY_ROTATION_PATH);
                draw_angles_from_buffer(ui, &self.robot_body_rotations);

                if let Some(measurements_value) = self
                    .calibration_measurements
                    .get_last_value()
                    .ok()
                    .flatten()
                {
                    ui.separator();
                    ui.label("Measurements");
                    if ui.button("Download").clicked() {
                        // save measurement to disk as json
                        let json = serde_json::to_string_pretty(&measurements_value).unwrap();
                        let path = "measurements.json";

                        std::fs::write(path, json).unwrap();
                    }
                }
            } else {
                self.primary_state
                    .get_last_value()
                    .ok()
                    .flatten()
                    .map(|primary_state| match primary_state {
                        PrimaryState::Calibration => ui.label("Calibration in progress"),
                        _ => ui.label(format!(
                            "Not yet calibrated, primary state: {:?}",
                            primary_state
                        )),
                    });
            }
        })
        .response
    }
}

fn serialize_and_call<V: serde::Serialize, T: FnOnce(serde_json::Value)>(data: V, callback: T) {
    match serde_json::to_value(data) {
        Ok(value) => {
            callback(value);
        }
        Err(error) => error!("failed to serialize parameter value: {error:#?}"),
    }
}

fn draw_group(ui: &mut Ui, label: &str, rotations_radians: (f32, f32, f32), nao: &Nao, path: &str) {
    let rotations_degrees = [
        rotations_radians.0,
        rotations_radians.1,
        rotations_radians.2,
    ]
    .map(|radians: f32| radians.to_degrees());
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.button("Save to repo").clicked() {
            serialize_and_call(rotations_degrees, |value| {
                nao.store_parameters(path, value, Scope::default_head())
                    .log_err();
            });
        }
        if ui.button("Set in Nao").clicked() {
            serialize_and_call(rotations_degrees, |value| {
                nao.write(format!("parameters.{path}"), TextOrBinary::Text(value));
            });
        }
    });
    draw_angles(ui, &rotations_degrees, "Calibrated");
}

fn draw_angles_from_buffer(ui: &mut Ui, current_values: &BufferHandle<Vector3<f32>>) {
    if let Some(value) = current_values.get_last_value().ok().flatten() {
        draw_angles(ui, &[value.x, value.y, value.z], "Current");
    }
}
fn draw_angles(ui: &mut Ui, rotations_degrees: &[f32; 3], sublabel: &str) {
    ui.label(format!(
        "{sublabel}: [{0:.2}°, {1:.2}°, {2:.2}°]",
        rotations_degrees[0], rotations_degrees[1], rotations_degrees[2]
    ));
}
