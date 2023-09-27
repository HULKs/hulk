use corrections::Corrections;
use levenberg_marquardt::LevenbergMarquardt;
use measurement::Measurement;
use problem::CalibrationProblem;
use types::field_dimensions::FieldDimensions;

pub mod corrections;
pub mod jacobian;
pub mod lines;
pub mod measurement;
pub mod problem;
pub mod residuals;

pub fn solve(
    initial_corrections: Corrections,
    measurements: Vec<Measurement>,
    field_dimensions: FieldDimensions,
) -> Corrections {
    let problem = CalibrationProblem::new(initial_corrections, measurements, field_dimensions);
    let (result, report) = LevenbergMarquardt::new().minimize(problem);
    println!("Report: {report:?}");
    let corrections = result.get_corrections();
    println!("Corrections: {corrections:?}");
    corrections
}
