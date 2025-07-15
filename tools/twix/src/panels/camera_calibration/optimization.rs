use std::sync::Arc;

use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};

use calibration::{
    corrections::Corrections,
    goal_and_penalty_box::{LineType, Measurement, Residuals},
    problem::CalibrationProblem,
};
use communication::messages::TextOrBinary;
use coordinate_systems::Pixel;
use geometry::line_segment::LineSegment;
use levenberg_marquardt::{LevenbergMarquardt, MinimizationReport};
use linear_algebra::Isometry2;
use parameters::directory::Scope;
use projection::camera_matrix::CameraMatrix;
use serde_json::Value;
use types::{camera_position::CameraPosition, field_dimensions::FieldDimensions};

use crate::{nao::Nao, value_buffer::BufferHandle};

const ROBOT_CORRECTION_PATH: &str =
    "parameters.camera_matrix_parameters.calibration.correction_in_robot";
const CAMERA_TOP_CORRECTION_PATH: &str =
    "parameters.camera_matrix_parameters.calibration.correction_in_camera_top";
const CAMERA_BOTTOM_CORRECTION_PATH: &str =
    "parameters.camera_matrix_parameters.calibration.correction_in_camera_bottom";

pub struct SemiAutomaticCalibrationContext {
    nao: Arc<Nao>,
    state: OptimizationState,

    top_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    bottom_camera_correction: BufferHandle<nalgebra::Vector3<f32>>,
    robot_correction: BufferHandle<nalgebra::Vector3<f32>>,
    field_dimensions: BufferHandle<FieldDimensions>,
}

#[derive(Clone, Copy, Debug)]
pub struct DrawnLine {
    pub line_segment: LineSegment<Pixel>,
    pub line_type: LineType,
}

#[derive(Clone, Debug)]
pub struct SavedMeasurement {
    pub camera_position: CameraPosition,
    pub camera_matrix: CameraMatrix,
    pub drawn_lines: Vec<DrawnLine>,
}

enum OptimizationState {
    NotOptimized,
    Optimized {
        corrections: Corrections,
        report: MinimizationReport<f32>,
    },
}

impl SemiAutomaticCalibrationContext {
    pub fn new(nao: Arc<Nao>) -> Self {
        let top_camera_correction = nao.subscribe_value(ROBOT_CORRECTION_PATH);
        let bottom_camera_correction = nao.subscribe_value(CAMERA_BOTTOM_CORRECTION_PATH);
        let robot_correction = nao.subscribe_value(CAMERA_TOP_CORRECTION_PATH);
        let field_dimensions = nao.subscribe_value("parameters.field_dimensions");

        Self {
            nao,
            state: OptimizationState::NotOptimized,
            top_camera_correction,
            bottom_camera_correction,
            robot_correction,
            field_dimensions,
        }
    }

    pub fn optimization_report(&self) -> Option<&MinimizationReport<f32>> {
        match &self.state {
            OptimizationState::NotOptimized => None,
            OptimizationState::Optimized { report, .. } => Some(report),
        }
    }

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

    pub fn run_optimization(&mut self, measurements: Vec<SavedMeasurement>) -> Result<()> {
        let initial_corrections = self.corrections()?;
        let field_dimensions = self
            .field_dimensions
            .get_last_value()?
            .wrap_err("failed to get field dimensions")?;

        let (corrections, report) = optimize(initial_corrections, field_dimensions, measurements)
            .wrap_err("failed to optimize")?;

        self.apply_corrections(corrections, |path, value| {
            self.nao.write(path, TextOrBinary::Text(value));
            Ok(())
        })?;
        self.state = OptimizationState::Optimized {
            corrections,
            report,
        };
        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        self.state = OptimizationState::NotOptimized;
        self.apply_corrections(Corrections::default(), |path, value| {
            self.nao.write(path, TextOrBinary::Text(value));
            Ok(())
        })
    }

    pub fn save_to_head(&self) -> Result<()> {
        if let OptimizationState::Optimized { corrections, .. } = &self.state {
            return self.apply_corrections(*corrections, |path, value| {
                let parameter_path = path.strip_prefix("parameters.").wrap_err("invalid path")?;
                self.nao
                    .store_parameters(parameter_path, value, Scope::default_head())
            });
        }
        bail!("optimization is not done yet")
    }
}

fn optimize(
    initial_corrections: Corrections,
    field_dimensions: FieldDimensions,
    measurements: Vec<SavedMeasurement>,
) -> Result<(Corrections, MinimizationReport<f32>)> {
    let measurements = measurements
        .into_iter()
        .flat_map(|measurement| {
            measurement
                .drawn_lines
                .into_iter()
                .map(move |line| Measurement {
                    camera_matrix: measurement.camera_matrix.clone(),
                    line_type: line.line_type,
                    line_segment: line.line_segment,
                    position: measurement.camera_position,
                    field_to_ground: Isometry2::identity(),
                })
        })
        .collect();

    let problem =
        CalibrationProblem::<Residuals>::new(initial_corrections, measurements, field_dimensions);
    let (result, report) = LevenbergMarquardt::new().minimize(problem);
    let optimized_corrections = result.get_corrections();
    Ok((optimized_corrections, report))
}
