use itertools::Itertools;
use levenberg_marquardt::LevenbergMarquardt;

use types::field_dimensions::FieldDimensions;

use corrections::Corrections;
use problem::CalibrationProblem;
use residuals::{calculate_residuals_from_parameters, CalculateResiduals};

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
    MeasurementResidualsType::Measurement: Clone,
{
    let problem = CalibrationProblem::<MeasurementResidualsType>::new(
        initial_corrections,
        measurements.clone(),
        field_dimensions,
    );

    let (result, report) = LevenbergMarquardt::new().minimize(problem);
    println!("Report: {report:?}");

    // let residuals = calculate_residuals_from_parameters(
    //     &result.get_corrections(),
    //     &measurements,
    //     &field_dimensions,
    // );
    // if let Some(residuals) = residuals {
    //     // println!("residuals: {residuals:?}");
    //     _simple_hist(residuals.as_slice(), 20);
    // }

    let corrections = result.get_corrections();
    // println!("Corrections: {corrections:?}");

    let euler_top = corrections.correction_in_camera_top.inner.euler_angles();
    let euler_bottom = corrections.correction_in_camera_bottom.inner.euler_angles();
    let euler_robot = corrections.correction_in_robot.inner.euler_angles();
    println!(
        "Euler Angles, top: {:?}deg, bottom: {:?}deg, robot: {:?}deg",
        (
            euler_top.0.to_degrees(),
            euler_top.1.to_degrees(),
            euler_top.2.to_degrees()
        ),
        (
            euler_bottom.0.to_degrees(),
            euler_bottom.1.to_degrees(),
            euler_bottom.2.to_degrees()
        ),
        (
            euler_robot.0.to_degrees(),
            euler_robot.1.to_degrees(),
            euler_robot.2.to_degrees()
        ),
    );
    corrections
}

fn _simple_hist(input: &[f32], bins: usize) -> Vec<u32> {
    let min_max = input.iter().copied().minmax().into_option().unwrap();

    let bin_size = min_max.1 - min_max.0;

    let mut histogram = vec![0; bins];
    for distance in input {
        let bin = (distance - min_max.0) / bin_size;
        histogram[bin as usize] += 1;
    }

    println!(
        "range: [{}, {}], histogram: {:?}",
        min_max.0, min_max.1, histogram
    );
    histogram
}
