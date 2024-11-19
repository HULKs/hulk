use std::sync::Arc;

use eframe::egui::{Response, Ui, Widget};
use log::error;
use nalgebra::Vector3;
use serde_json::Value;

use calibration::corrections::Corrections;
use communication::messages::TextOrBinary;
use parameters::directory::Scope;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct AutomaticCameraCalibrationExportPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<Vector3<f32>>,
    bottom_camera: BufferHandle<Vector3<f32>>,
    calibration_corrections: BufferHandle<Value>,
}

impl Panel for AutomaticCameraCalibrationExportPanel {
    const NAME: &'static str = "Automatic Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let top_camera = nao
            .subscribe_value("parameters.camera_matrix_parameters.vision_top.extrinsic_rotations");
        let bottom_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_bottom.extrinsic_rotations",
        );
        let calibration_corrections =
            nao.subscribe_json("Control.additional_outputs.last_calibration_corrections");

        Self {
            nao,
            top_camera,
            bottom_camera,
            calibration_corrections,
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

                draw_group(
                    ui,
                    "Top",
                    top_angles,
                    &self.nao,
                    "camera_matrix_parameters.vision_top.extrinsic_rotations",
                );
                draw_angles_from_buffer(ui, &self.top_camera);
                ui.separator();

                draw_group(
                    ui,
                    "Bottom",
                    bottom_angles,
                    &self.nao,
                    "camera_matrix_parameters.vision_bottom.extrinsic_rotations",
                );
                draw_angles_from_buffer(ui, &self.bottom_camera);
                ui.separator();

                ui.label("Body");
                draw_angles(ui, body_angles, "Calibrated");
            } else {
                ui.label("Not yet calibrated");
            }
        })
        .response
    }
}

fn serialize_and_call<V: serde::Serialize, T: FnOnce(serde_json::Value) -> ()>(
    data: V,
    callback: T,
) {
    match serde_json::to_value(data) {
        Ok(value) => {
            callback(value);
        }
        Err(error) => error!("failed to serialize parameter value: {error:#?}"),
    }
}

fn draw_group(ui: &mut Ui, label: &str, rotations: (f32, f32, f32), nao: &Nao, path: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.button("Save to repo").clicked() {
            serialize_and_call(rotations, |value| {
                nao.store_parameters(path, value, Scope::default_head())
                    .log_err();
            });
        }
        if ui.button("Set in Nao").clicked() {
            serialize_and_call(rotations, |value| {
                nao.write(path, TextOrBinary::Text(value));
            });
        }
    });
    draw_angles(ui, rotations, "Calibrated");
}

fn draw_angles_from_buffer(ui: &mut Ui, current_values: &BufferHandle<Vector3<f32>>) {
    if let Some(value) = current_values.get_last_value().ok().flatten() {
        draw_angles(ui, (value.x, value.y, value.z), "Current");
    }
}
fn draw_angles(ui: &mut Ui, rotations: (f32, f32, f32), sublabel: &str) {
    ui.label(format!(
        "{sublabel}: [{0:.2}°, {1:.2}°, {2:.2}°]",
        rotations.0.to_degrees(),
        rotations.1.to_degrees(),
        rotations.2.to_degrees()
    ));
}
