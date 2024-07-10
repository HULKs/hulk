use std::sync::Arc;

use communication::messages::TextOrBinary;
use eframe::egui::{Response, Slider, Ui, Widget, WidgetText};
use log::error;
use nalgebra::Vector3;
use parameters::directory::Scope;
use serde_json::Value;

use crate::{log_error::LogError, nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<Vector3<f32>>,
    bottom_camera: BufferHandle<Vector3<f32>>,
}

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Manual Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let top_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_top.extrinsic_rotations".to_string(),
        );
        let bottom_camera = nao.subscribe_value(
            "parameters.camera_matrix_parameters.vision_bottom.extrinsic_rotations".to_string(),
        );

        Self {
            nao,
            top_camera,
            bottom_camera,
        }
    }
}

impl Widget for &mut ManualCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        ui.vertical(|ui| {
            if let Ok(Some(value)) = self.top_camera.get_last_value() {
                draw_calibration_ui(
                    ui,
                    "Top Camera",
                    value,
                    &self.nao,
                    "parameters.camera_matrix_parameters.vision_top.extrinsic_rotations",
                );
            }
            ui.separator();
            if let Ok(Some(value)) = self.bottom_camera.get_last_value() {
                draw_calibration_ui(
                    ui,
                    "Bottom Camera",
                    value,
                    &self.nao,
                    "parameters.camera_matrix_parameters.vision_bottom.extrinsic_rotations",
                );
            }
        })
        .response
    }
}

fn draw_calibration_ui(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    rotations: Vector3<f32>,
    nao: &Nao,
    path: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui.button("Save to Head").clicked() {
            let serialized = serde_json::to_value(rotations);
            match serialized {
                Ok(value) => {
                    nao.store_parameters(path, value, Scope::current_head())
                        .log_err();
                }
                Err(error) => error!("failed to serialize parameter value: {error:#?}"),
            }
        }
    });
    let range = -15.0..=15.0;
    let mut roll = rotations.x;
    let response = ui.add(
        Slider::new(&mut roll, range.clone())
            .text("Roll")
            .smart_aim(false),
    );
    if response.changed() {
        nao.write(
            format!("{path}.x"),
            TextOrBinary::Text(serde_json::to_value(roll).unwrap()),
        );
    }
    let mut pitch = rotations.y;
    let response = ui.add(
        Slider::new(&mut pitch, range.clone())
            .text("Pitch")
            .smart_aim(false),
    );
    if response.changed() {
        nao.write(
            format!("{path}.y"),
            TextOrBinary::Text(serde_json::to_value(pitch).unwrap()),
        );
    }
    let mut yaw = rotations.z;
    let response = ui.add(Slider::new(&mut yaw, range).text("Yaw").smart_aim(false));
    if response.changed() {
        nao.write(
            format!("{path}.z"),
            TextOrBinary::Text(serde_json::to_value(yaw).unwrap()),
        );
    }
}
