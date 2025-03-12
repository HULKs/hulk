use eframe::egui::{Response, Ui, Widget};

use crate::panel::Panel;

pub struct SemiAutomaticCameraCalibrationPanel {}

impl Panel for SemiAutomaticCameraCalibrationPanel {
    const NAME: &'static str = "Semi-Automatic Camera Calibration";

    fn new(_nao: std::sync::Arc<crate::nao::Nao>, _value: Option<&serde_json::Value>) -> Self {
        todo!()
    }
}

impl Widget for &mut SemiAutomaticCameraCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
