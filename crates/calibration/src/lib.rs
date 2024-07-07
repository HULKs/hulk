use corrections::Corrections;
use levenberg_marquardt::LevenbergMarquardt;
use problem::CalibrationProblem;
use residuals::CalculateResiduals;
use types::field_dimensions::FieldDimensions;

pub mod center_circle;
pub mod corrections;
pub mod goal_box;
pub mod jacobian;
pub mod problem;
pub mod residuals;

pub fn solve<MeasurementResidualsType>(
    initial_corrections: Corrections,
    measurements: Vec<MeasurementResidualsType::Measurement>,
    field_dimensions: FieldDimensions,
) -> Corrections
where
    MeasurementResidualsType: CalculateResiduals,
    Vec<f32>: From<MeasurementResidualsType>,
{
    let problem = CalibrationProblem::<MeasurementResidualsType>::new(
        initial_corrections,
        measurements,
        field_dimensions,
    );
    let (result, report) = LevenbergMarquardt::new().minimize(problem);
    println!("Report: {report:?}");
    let corrections = result.get_corrections();
    println!("Corrections: {corrections:?}");
    corrections
}
