use std::sync::Arc;

use calibration::{
    corrections::Corrections,
    goal_and_penalty_box::{LineType, Measurement, Residuals},
    problem::CalibrationProblem,
};
use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use communication::messages::TextOrBinary;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Response, RichText, Ui, Widget};
use geometry::line_segment::LineSegment;
use levenberg_marquardt::{LevenbergMarquardt, MinimizationReport};
use linear_algebra::{point, Isometry2};
use projection::camera_matrix::CameraMatrix;
use serde_json::Value;
use types::{camera_position::CameraPosition, field_dimensions::FieldDimensions};

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

const ROBOT_CORRECTION_PATH: &'static str =
    "parameters.camera_matrix_parameters.calibration.correction_in_robot";
const CAMERA_TOP_CORRECTION_PATH: &'static str =
    "parameters.camera_matrix_parameters.calibration.correction_in_camera_top";
const CAMERA_BOTTOM_CORRECTION_PATH: &'static str =
    "parameters.camera_matrix_parameters.calibration.correction_in_camera_bottom";

pub struct SemiAutomaticCalibrationPanel {
    nao: Arc<Nao>,
    top_camera: BufferHandle<CameraMatrix>,
    bottom_camera: BufferHandle<CameraMatrix>,
    last_optimization_state: Option<Result<(Corrections, MinimizationReport<f32>)>>,

    top_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    bottom_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    robot_correction: BufferHandle<nalgebra::Vector3<f32>>,
    field_dimensions: BufferHandle<FieldDimensions>,
}

impl Panel for SemiAutomaticCalibrationPanel {
    const NAME: &'static str = "Semi-Automatic Calibration";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let top_camera = nao.subscribe_value("Control.main_outputs.camera_matrices.top");
        let bottom_camera = nao.subscribe_value("Control.main_outputs.camera_matrices.bottom");

        let top_camera_correction = nao.subscribe_value(ROBOT_CORRECTION_PATH);
        let bottom_camera_correction = nao.subscribe_value(CAMERA_BOTTOM_CORRECTION_PATH);
        let robot_correction = nao.subscribe_value(CAMERA_TOP_CORRECTION_PATH);
        let field_dimensions = nao.subscribe_value("parameters.field_dimensions");

        Self {
            nao,
            top_camera,
            bottom_camera,
            last_optimization_state: None,
            top_camera_correction,
            bottom_camera_correction,
            robot_correction,
            field_dimensions,
        }
    }
}

impl SemiAutomaticCalibrationPanel {
    fn corrections(&self) -> Result<Corrections> {
        let correction_in_robot = self
            .robot_correction
            .get_last_value()?
            .wrap_err("failed to get robot correction")?;
        let correction_in_camera_top = self
            .top_camera_correction
            .get_last_value()?
            .wrap_err("failed to get camera top correction")?;
        let correction_in_camera_bottom = self
            .bottom_camera_correction
            .get_last_value()?
            .wrap_err("failed to get camera bottom correction")?;

        let correction_in_robot = nalgebra::Rotation3::from_euler_angles(
            correction_in_robot.x,
            correction_in_robot.y,
            correction_in_robot.z,
        );
        let correction_in_camera_top = nalgebra::Rotation3::from_euler_angles(
            correction_in_camera_top.x,
            correction_in_camera_top.y,
            correction_in_camera_top.z,
        );
        let correction_in_camera_bottom = nalgebra::Rotation3::from_euler_angles(
            correction_in_camera_bottom.x,
            correction_in_camera_bottom.y,
            correction_in_camera_bottom.z,
        );

        Ok(Corrections {
            correction_in_robot,
            correction_in_camera_top,
            correction_in_camera_bottom,
        })
    }

    fn apply_corrections(&self, corrections: Corrections) -> Result<()> {
        let (x, y, z) = corrections.correction_in_robot.euler_angles();
        self.nao.write(
            ROBOT_CORRECTION_PATH,
            TextOrBinary::Text(serde_json::to_value([x, y, z])?),
        );
        let (x, y, z) = corrections.correction_in_camera_top.euler_angles();
        self.nao.write(
            CAMERA_TOP_CORRECTION_PATH,
            TextOrBinary::Text(serde_json::to_value([x, y, z])?),
        );
        let (x, y, z) = corrections.correction_in_camera_bottom.euler_angles();
        self.nao.write(
            CAMERA_BOTTOM_CORRECTION_PATH,
            TextOrBinary::Text(serde_json::to_value([x, y, z])?),
        );
        Ok(())
    }

    fn optimize(
        &self,
        initial_corrections: Corrections,
        drawn_lines: Vec<(LineType, LineSegment<Pixel>)>,
    ) -> Result<(Corrections, MinimizationReport<f32>)> {
        let field_dimensions = self
            .field_dimensions
            .get_last_value()?
            .wrap_err("failed to get field dimensions")?;
        let top_camera = self
            .top_camera
            .get_last_value()?
            .wrap_err("failed to get top camera")?;
        let measurements = drawn_lines
            .into_iter()
            .map(|(line_type, line)| Measurement {
                line_type,
                line,
                camera_matrix: top_camera.clone(),
                position: CameraPosition::Top,
                field_to_ground: Isometry2::identity(),
            })
            .collect();
        let problem = CalibrationProblem::<Residuals>::new(
            initial_corrections,
            measurements,
            field_dimensions,
        );
        let (result, report) = LevenbergMarquardt::new().minimize(problem);
        let optimized_corrections = result.get_corrections();
        Ok((optimized_corrections, report))
    }

    fn should_start_calibration(&self, ui: &mut Ui) -> Option<Result<Corrections>> {
        if ui.button("Start Calibration").clicked() {
            Some(self.corrections())
        } else {
            None
        }
    }

    fn start_optimization(
        &self,
        initial_corrections: Result<Corrections>,
        drawn_lines: Vec<(LineType, LineSegment<Pixel>)>,
    ) -> Result<(Corrections, MinimizationReport<f32>)> {
        let initial_corrections = initial_corrections?;

        let result = self.optimize(initial_corrections, drawn_lines);
        if let Ok((corrections, _)) = result {
            self.apply_corrections(corrections)?;
        }
        result
    }
}

impl Widget for &mut SemiAutomaticCalibrationPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.group(|ui| {
            let drawn_lines = vec![
                (
                    LineType::FrontPenaltyArea,
                    LineSegment::new(point![151.0, 80.0], point![640.0, 86.0]),
                ),
                (
                    LineType::LeftPenaltyArea,
                    LineSegment::new(point![151.0, 80.0], point![262.0, 29.5]),
                ),
            ];

            let result = if let Some(corrections) = self.should_start_calibration(ui) {
                Some(self.start_optimization(corrections, drawn_lines))
            } else {
                None
            };

            if result.is_some() {
                self.last_optimization_state = result;
            }

            if let Some(Err(error)) = &self.last_optimization_state {
                ui.label(RichText::new(error.to_string()).color(Color32::RED));
            }

            if let Some(Ok((_, report))) = &self.last_optimization_state {
                ui.label(format!("Iterations: {}", report.number_of_evaluations));
                ui.label(format!("Residual: {}", report.objective_function));
                ui.label(format!("Termination reason: {:?}", report.termination));
            }
        })
        .response
    }
}
