use std::{ops::RangeInclusive, sync::Arc};

use communication::messages::TextOrBinary;
use eframe::egui::{Response, Slider, Ui, Widget, WidgetText};
use log::error;
use nalgebra::Vector3;
use parameters::directory::Scope;
use serde_json::Value;

use crate::{
    log_error::LogError, nao::Nao, panel::Panel, panels::CAMERA_EXTRINSICS_PATH,
    value_buffer::BufferHandle,
};

pub struct ManualCalibrationPanel {
    nao: Arc<Nao>,
    camera: BufferHandle<Vector3<f32>>,
}

impl Panel for ManualCalibrationPanel {
    const NAME: &'static str = "Manual Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let camera = nao.subscribe_value(format!("parameters.{CAMERA_EXTRINSICS_PATH}"));

        Self { nao, camera }
    }
}

impl Widget for &mut ManualCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.style_mut().spacing.slider_width = ui.available_size().x - 250.0;
        ui.vertical(|ui| {
            if let Ok(Some(value)) = self.camera.get_last_value() {
                draw_calibration_ui(ui, "Camera", value, &self.nao, CAMERA_EXTRINSICS_PATH);
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
                    nao.store_parameters(path, value, Scope::default_head())
                        .log_err();
                }
                Err(error) => error!("manual camera calibration panel: failed to serialize parameter value: {error:#?}"),
            }
        }
    });

    // Note: Roll, pitch, yaw are swapped to the actual way an airplane fly in the following section for the UI
    let range = -15.0..=15.0;
    let mut roll = rotations.z;
    let response = show_slider_in_degree(ui, &mut roll, range.clone(), "Roll");
    if response.changed() {
        nao.write(
            format!("parameters.{path}.z"),
            TextOrBinary::Text(serde_json::to_value(roll).unwrap()),
        );
    }
    let mut pitch = rotations.x;
    let response = show_slider_in_degree(ui, &mut pitch, range.clone(), "Pitch");
    if response.changed() {
        nao.write(
            format!("parameters.{path}.x"),
            TextOrBinary::Text(serde_json::to_value(pitch).unwrap()),
        );
    }
    let mut yaw = rotations.y;
    let response = show_slider_in_degree(ui, &mut yaw, range, "Yaw");
    if response.changed() {
        nao.write(
            format!("parameters.{path}.y"),
            TextOrBinary::Text(serde_json::to_value(yaw).unwrap()),
        );
    }
}

fn show_slider_in_degree(
    ui: &mut Ui,
    angle_radians: &mut f32,
    range_degrees: RangeInclusive<f32>,
    name: &str,
) -> Response {
    let mut angle_degrees = angle_radians.to_degrees();
    let response = ui.add(
        Slider::new(&mut angle_degrees, range_degrees)
            .text(name)
            .smart_aim(false),
    );
    *angle_radians = angle_degrees.to_radians();
    response
}
