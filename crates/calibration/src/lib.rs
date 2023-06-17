use corrections::Corrections;
use levenberg_marquardt::LevenbergMarquardt;
use lines::Lines;
use problem::CalibrationProblem;
use types::{CameraMatrices, FieldDimensions};

pub mod corrections;
pub mod jacobian;
pub mod lines;
pub mod problem;
pub mod residuals;

pub fn solve(
    initial_corrections: Corrections,
    original: CameraMatrices,
    measurements: &[Lines],
    field_dimensions: FieldDimensions,
) -> Corrections {
    let problem = CalibrationProblem::new(
        initial_corrections,
        original,
        measurements,
        field_dimensions,
    );
    let (result, report) = LevenbergMarquardt::new().minimize(problem);
    println!("Report: {report:?}");
    let corrections = result.get_corrections();
    println!("Corrections: {corrections:?}");
    corrections
}
