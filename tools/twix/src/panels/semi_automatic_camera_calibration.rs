use std::sync::Arc;

use calibration::{
    corrections::Corrections,
    goal_and_penalty_box::{LineType, Measurement, Residuals},
    problem::CalibrationProblem,
};
use color_eyre::eyre::ContextCompat;
use color_eyre::{Report, Result};
use communication::messages::TextOrBinary;
use coordinate_systems::Pixel;
use eframe::egui::{Align::Center, Color32, Layout, Response, RichText, Ui, Widget};
use geometry::line_segment::LineSegment;
use levenberg_marquardt::{LevenbergMarquardt, MinimizationReport};
use linear_algebra::{point, Isometry2};
use parameters::directory::Scope;
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
    state: OptimizationState,

    top_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    bottom_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    robot_correction: BufferHandle<nalgebra::Vector3<f32>>,
    field_dimensions: BufferHandle<FieldDimensions>,
}

enum OptimizationState {
    NotOptimized,
    OptimizationRequested {
        initial_corrections: Corrections,
    },
    Optimized {
        corrections: Corrections,
        report: MinimizationReport<f32>,
    },
    Error(Report),
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
            state: OptimizationState::NotOptimized,
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

    fn apply_corrections(
        &self,
        corrections: Corrections,
        save_function: impl Fn(&str, Value) -> Result<()>,
    ) -> Result<()> {
        let (x, y, z) = corrections.correction_in_robot.euler_angles();
        save_function(ROBOT_CORRECTION_PATH, serde_json::to_value([x, y, z])?)?;

        let (x, y, z) = corrections.correction_in_camera_top.euler_angles();
        save_function(CAMERA_TOP_CORRECTION_PATH, serde_json::to_value([x, y, z])?)?;

        let (x, y, z) = corrections.correction_in_camera_bottom.euler_angles();
        save_function(
            CAMERA_BOTTOM_CORRECTION_PATH,
            serde_json::to_value([x, y, z])?,
        )?;

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

    fn run_optimization(
        &self,
        initial_corrections: Corrections,
        drawn_lines: Vec<(LineType, LineSegment<Pixel>)>,
    ) -> Result<(Corrections, MinimizationReport<f32>)> {
        let result = self.optimize(initial_corrections, drawn_lines);
        if let Ok((corrections, _)) = result {
            self.apply_corrections(corrections, |path, value| {
                Ok(self.nao.write(path, TextOrBinary::Text(value)))
            })?;
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

            let (start_optimization_clicked, save_to_head_clicked) = ui
                .horizontal(|ui| {
                    (
                        ui.button("Start Optimization").clicked(),
                        ui.button("Save to head").clicked(),
                    )
                })
                .inner;

            if start_optimization_clicked {
                self.state = match self.corrections() {
                    Ok(initial_corrections) => OptimizationState::OptimizationRequested {
                        initial_corrections,
                    },
                    Err(error) => OptimizationState::Error(error),
                };
            }

            if let OptimizationState::OptimizationRequested {
                initial_corrections,
            } = self.state
            {
                self.state = match self.run_optimization(initial_corrections, drawn_lines) {
                    Ok((corrections, report)) => OptimizationState::Optimized {
                        corrections,
                        report,
                    },
                    Err(error) => OptimizationState::Error(error),
                };
            }

            match &self.state {
                OptimizationState::Optimized { report, .. } => {
                    ui.label(format!("Iterations: {}", report.number_of_evaluations));
                    ui.label(format!("Residual: {}", report.objective_function));
                    ui.label(format!("Termination reason: {:?}", report.termination));
                }
                OptimizationState::Error(error) => {
                    ui.label(RichText::new(error.to_string()).color(Color32::RED));
                }
                _ => (),
            }

            if save_to_head_clicked {
                if let OptimizationState::Optimized { corrections, .. } = &self.state {
                    let save_result = self.apply_corrections(corrections.clone(), |path, value| {
                        self.nao
                            .store_parameters(path, value, Scope::current_head())
                    });
                    if let Err(error) = save_result {
                        self.state = OptimizationState::Error(error);
                    }
                }
            }
        })
        .response
    }
}
