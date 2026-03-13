use nalgebra::{
    Complex, ComplexField, Isometry2, Matrix2, Matrix3, Matrix3x2, UnitComplex, Vector2, Vector3,
    vector,
};
use thiserror::Error;
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

const STATE_DIMENSION: usize = 3;
const CUBATURE_POINT_COUNT: usize = 2 * STATE_DIMENSION;
const CUBATURE_POINT_WEIGHT: f32 = 1.0 / CUBATURE_POINT_COUNT as f32;

type MeasurementMeanFunction = fn(&[Vector2<f32>]) -> Vector2<f32>;
type MeasurementCovarianceFunction = fn(Vector2<f32>, &[Vector2<f32>]) -> Matrix2<f32>;
type CrossCovarianceFunction =
    fn(Vector3<f32>, &[Vector3<f32>], &Vector2<f32>, &[Vector2<f32>]) -> Matrix3x2<f32>;
type ResidualFunction = fn(Vector2<f32>, Vector2<f32>) -> Vector2<f32>;

struct MeasurementUpdateModel {
    mean_from_measurements: MeasurementMeanFunction,
    covariance_from_measurements: MeasurementCovarianceFunction,
    cross_covariance_from_measurements: CrossCovarianceFunction,
    residual_function: ResidualFunction,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to compute the inverse of the covariance matrix")]
    Inverse,
    #[error("failed to compute the cholesky decomposition of the covariance matrix")]
    Cholesky,
}

pub trait PoseFilter {
    fn predict<StatePredictionFunction>(
        &mut self,
        state_prediction_function: StatePredictionFunction,
        process_noise: Matrix3<f32>,
    ) -> Result<(), Error>
    where
        StatePredictionFunction: Fn(Vector3<f32>) -> Vector3<f32>;

    fn update_with_1d_translation_and_rotation<MeasurementPredictionFunction>(
        &mut self,
        measurement: Vector2<f32>,
        measurement_noise: Matrix2<f32>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<(), Error>
    where
        MeasurementPredictionFunction: Fn(Vector3<f32>) -> Vector2<f32>;

    fn update_with_2d_translation<MeasurementPredictionFunction>(
        &mut self,
        measurement: Vector2<f32>,
        measurement_noise: Matrix2<f32>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<(), Error>
    where
        MeasurementPredictionFunction: Fn(Vector3<f32>) -> Vector2<f32>;

    fn as_isometry(&self) -> Isometry2<f32>;
}

impl PoseFilter for MultivariateNormalDistribution<STATE_DIMENSION> {
    fn predict<StatePredictionFunction>(
        &mut self,
        state_prediction_function: StatePredictionFunction,
        process_noise: Matrix3<f32>,
    ) -> Result<(), Error>
    where
        StatePredictionFunction: Fn(Vector3<f32>) -> Vector3<f32>,
    {
        let cubature_points = sample_cubature_points(self.mean, self.covariance)?;
        let predicted_cubature_points: Vec<_> = cubature_points
            .iter()
            .copied()
            .map(state_prediction_function)
            .collect();
        let state_mean = mean_from_3d_cubature_points(&predicted_cubature_points);
        let state_covariance =
            covariance_from_3d_cubature_points(state_mean, &predicted_cubature_points);
        self.mean = normalized_state_angle(state_mean);
        self.covariance = into_symmetric(state_covariance + process_noise);

        Ok(())
    }

    fn update_with_1d_translation_and_rotation<MeasurementPredictionFunction>(
        &mut self,
        measurement: Vector2<f32>,
        measurement_noise: Matrix2<f32>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<(), Error>
    where
        MeasurementPredictionFunction: Fn(Vector3<f32>) -> Vector2<f32>,
    {
        update_with_2d_measurement(
            self,
            measurement,
            measurement_noise,
            measurement_prediction_function,
            MeasurementUpdateModel {
                mean_from_measurements: mean_from_1d_translation_and_rotation_cubature_points,
                covariance_from_measurements:
                    covariance_from_1d_translation_and_rotation_cubature_points,
                cross_covariance_from_measurements:
                    cross_covariance_from_1d_translation_and_rotation_cubature_points,
                residual_function: translation_and_rotation_residual,
            },
        )
    }

    fn update_with_2d_translation<MeasurementPredictionFunction>(
        &mut self,
        measurement: Vector2<f32>,
        measurement_noise: Matrix2<f32>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<(), Error>
    where
        MeasurementPredictionFunction: Fn(Vector3<f32>) -> Vector2<f32>,
    {
        update_with_2d_measurement(
            self,
            measurement,
            measurement_noise,
            measurement_prediction_function,
            MeasurementUpdateModel {
                mean_from_measurements: mean_from_2d_translation_cubature_points,
                covariance_from_measurements: covariance_from_2d_translation_cubature_points,
                cross_covariance_from_measurements:
                    cross_covariance_from_2d_translation_cubature_points,
                residual_function: translation_residual,
            },
        )
    }

    fn as_isometry(&self) -> Isometry2<f32> {
        Isometry2::new(vector![self.mean.x, self.mean.y], self.mean.z)
    }
}

fn into_symmetric(matrix: Matrix3<f32>) -> Matrix3<f32> {
    0.5 * (matrix + matrix.transpose())
}

fn wrap_angle(angle: f32) -> f32 {
    UnitComplex::new(angle).angle()
}

fn angle_difference(measurement: f32, prediction: f32) -> f32 {
    wrap_angle(measurement - prediction)
}

fn normalized_state_angle(mut mean: Vector3<f32>) -> Vector3<f32> {
    mean.z = wrap_angle(mean.z);
    mean
}

fn translation_and_rotation_residual(
    measurement: Vector2<f32>,
    predicted_measurement_mean: Vector2<f32>,
) -> Vector2<f32> {
    vector![
        measurement.x - predicted_measurement_mean.x,
        angle_difference(measurement.y, predicted_measurement_mean.y)
    ]
}

fn translation_residual(
    measurement: Vector2<f32>,
    predicted_measurement_mean: Vector2<f32>,
) -> Vector2<f32> {
    measurement - predicted_measurement_mean
}

fn joseph_form_update(
    prior_covariance: Matrix3<f32>,
    kalman_gain: Matrix3x2<f32>,
    predicted_measurements_cross_covariance: Matrix3x2<f32>,
    innovation_covariance: Matrix2<f32>,
) -> Matrix3<f32> {
    prior_covariance
        - kalman_gain * predicted_measurements_cross_covariance.transpose()
        - predicted_measurements_cross_covariance * kalman_gain.transpose()
        + kalman_gain * innovation_covariance * kalman_gain.transpose()
}

fn update_with_2d_measurement(
    state: &mut MultivariateNormalDistribution<STATE_DIMENSION>,
    measurement: Vector2<f32>,
    measurement_noise: Matrix2<f32>,
    measurement_prediction_function: impl Fn(Vector3<f32>) -> Vector2<f32>,
    update_model: MeasurementUpdateModel,
) -> Result<(), Error> {
    let cubature_points = sample_cubature_points(state.mean, state.covariance)?;
    let predicted_measurements: Vec<_> = cubature_points
        .iter()
        .copied()
        .map(measurement_prediction_function)
        .collect();
    let predicted_measurement_mean = (update_model.mean_from_measurements)(&predicted_measurements);
    let predicted_measurement_covariance = (update_model.covariance_from_measurements)(
        predicted_measurement_mean,
        &predicted_measurements,
    );
    let predicted_measurements_cross_covariance = (update_model.cross_covariance_from_measurements)(
        state.mean,
        &cubature_points,
        &predicted_measurement_mean,
        &predicted_measurements,
    );
    let innovation_covariance = predicted_measurement_covariance + measurement_noise;
    let kalman_gain = predicted_measurements_cross_covariance
        * innovation_covariance.try_inverse().ok_or(Error::Inverse)?;
    let residual = (update_model.residual_function)(measurement, predicted_measurement_mean);

    state.mean = normalized_state_angle(state.mean + kalman_gain * residual);
    state.covariance = into_symmetric(joseph_form_update(
        state.covariance,
        kalman_gain,
        predicted_measurements_cross_covariance,
        innovation_covariance,
    ));

    Ok(())
}

fn sample_cubature_points(
    mean: Vector3<f32>,
    covariance: Matrix3<f32>,
) -> Result<[Vector3<f32>; CUBATURE_POINT_COUNT], Error> {
    let covariance_cholesky = covariance.cholesky().ok_or(Error::Cholesky)?;
    // Third-degree cubature rule for a STATE_DIMENSION-dimensional state:
    // +/- sqrt(STATE_DIMENSION) along each Cholesky axis.
    let covariance_square_root = (STATE_DIMENSION as f32).sqrt() * covariance_cholesky.l();

    let cubature_points = [
        mean + covariance_square_root.column(0),
        mean - covariance_square_root.column(0),
        mean + covariance_square_root.column(1),
        mean - covariance_square_root.column(1),
        mean + covariance_square_root.column(2),
        mean - covariance_square_root.column(2),
    ];
    Ok(cubature_points)
}

fn mean_from_3d_cubature_points(points: &[Vector3<f32>]) -> Vector3<f32> {
    let mut mean = Vector2::zeros();
    let mut mean_angle = Complex::new(0.0, 0.0);
    for point in points {
        mean += point.xy();
        mean_angle += Complex::new(point.z.cos(), point.z.sin());
    }
    mean *= CUBATURE_POINT_WEIGHT;
    vector![mean.x, mean.y, mean_angle.argument()]
}

fn mean_from_1d_translation_and_rotation_cubature_points(points: &[Vector2<f32>]) -> Vector2<f32> {
    let mut mean_x = 0.0;
    let mut mean_angle = Complex::new(0.0, 0.0);
    for point in points {
        mean_x += point.x;
        mean_angle += Complex::new(point.y.cos(), point.y.sin());
    }
    mean_x *= CUBATURE_POINT_WEIGHT;
    vector![mean_x, mean_angle.argument()]
}

fn mean_from_2d_translation_cubature_points(points: &[Vector2<f32>]) -> Vector2<f32> {
    let mut mean = Vector2::zeros();
    for point in points {
        mean += point;
    }
    mean *= CUBATURE_POINT_WEIGHT;
    mean
}

fn covariance_from_3d_cubature_points(
    mean: Vector3<f32>,
    cubature_points: &[Vector3<f32>],
) -> Matrix3<f32> {
    cubature_points
        .iter()
        .map(|point| {
            vector![
                point.x - mean.x,
                point.y - mean.y,
                (UnitComplex::new(point.z) / UnitComplex::new(mean.z)).angle()
            ]
        })
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<Matrix3<f32>>()
        * CUBATURE_POINT_WEIGHT
}

fn covariance_from_1d_translation_and_rotation_cubature_points(
    mean: Vector2<f32>,
    cubature_points: &[Vector2<f32>],
) -> Matrix2<f32> {
    cubature_points
        .iter()
        .map(|point| {
            vector![
                point.x - mean.x,
                (UnitComplex::new(point.y) / UnitComplex::new(mean.y)).angle()
            ]
        })
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<Matrix2<f32>>()
        * CUBATURE_POINT_WEIGHT
}

fn covariance_from_2d_translation_cubature_points(
    mean: Vector2<f32>,
    cubature_points: &[Vector2<f32>],
) -> Matrix2<f32> {
    cubature_points
        .iter()
        .map(|point| point - mean)
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<Matrix2<f32>>()
        * CUBATURE_POINT_WEIGHT
}

fn cross_covariance_from_1d_translation_and_rotation_cubature_points(
    state_mean: Vector3<f32>,
    state_cubature_points: &[Vector3<f32>],
    &measurement_mean: &Vector2<f32>,
    measurement_cubature_points: &[Vector2<f32>],
) -> Matrix3x2<f32> {
    assert!(state_cubature_points.len() == measurement_cubature_points.len());
    state_cubature_points
        .iter()
        .zip(measurement_cubature_points.iter())
        .map(|(state, measurement)| {
            vector![
                state.x - state_mean.x,
                state.y - state_mean.y,
                (UnitComplex::new(state.z) / UnitComplex::new(state_mean.z)).angle()
            ] * vector![
                measurement.x - measurement_mean.x,
                (UnitComplex::new(measurement.y) / UnitComplex::new(measurement_mean.y)).angle()
            ]
            .transpose()
        })
        .sum::<Matrix3x2<f32>>()
        * CUBATURE_POINT_WEIGHT
}

fn cross_covariance_from_2d_translation_cubature_points(
    state_mean: Vector3<f32>,
    state_cubature_points: &[Vector3<f32>],
    &measurement_mean: &Vector2<f32>,
    measurement_cubature_points: &[Vector2<f32>],
) -> Matrix3x2<f32> {
    assert!(state_cubature_points.len() == measurement_cubature_points.len());
    state_cubature_points
        .iter()
        .zip(measurement_cubature_points.iter())
        .map(|(state, measurement)| {
            vector![
                state.x - state_mean.x,
                state.y - state_mean.y,
                (UnitComplex::new(state.z) / UnitComplex::new(state_mean.z)).angle()
            ] * (measurement - measurement_mean).transpose()
        })
        .sum::<Matrix3x2<f32>>()
        * CUBATURE_POINT_WEIGHT
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use approx::assert_relative_eq;
    use nalgebra::{matrix, vector};

    use super::{CUBATURE_POINT_COUNT, PoseFilter, sample_cubature_points, wrap_angle};
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    #[test]
    fn identity_prediction_preserves_covariance_without_process_noise() {
        let initial_covariance = matrix![
            0.4, 0.1, 0.0;
            0.1, 0.3, 0.0;
            0.0, 0.0, 0.2
        ];
        let mut state = MultivariateNormalDistribution {
            mean: vector![1.0, -0.5, 0.3],
            covariance: initial_covariance,
        };

        state
            .predict(
                |state| state,
                matrix![0.0, 0.0, 0.0; 0.0, 0.0, 0.0; 0.0, 0.0, 0.0],
            )
            .unwrap();

        assert_relative_eq!(state.covariance, initial_covariance, epsilon = 1.0e-5);
    }

    #[test]
    fn repeated_identity_predictions_accumulate_process_noise() {
        let process_noise = matrix![
            0.02, 0.0, 0.0;
            0.0, 0.03, 0.0;
            0.0, 0.0, 0.01
        ];
        let initial_covariance = matrix![
            0.4, 0.0, 0.0;
            0.0, 0.3, 0.0;
            0.0, 0.0, 0.2
        ];
        let mut state = MultivariateNormalDistribution {
            mean: vector![0.0, 0.0, 0.0],
            covariance: initial_covariance,
        };

        for _ in 0..3 {
            state.predict(|state| state, process_noise).unwrap();
        }

        assert_relative_eq!(
            state.covariance,
            initial_covariance + 3.0 * process_noise,
            epsilon = 1.0e-5
        );
    }

    #[test]
    fn cubature_sampler_returns_expected_number_of_points() {
        let cubature_points = sample_cubature_points(
            vector![0.0, 0.0, 0.0],
            matrix![
                1.0, 0.0, 0.0;
                0.0, 1.0, 0.0;
                0.0, 0.0, 1.0
            ],
        )
        .unwrap();

        assert_eq!(cubature_points.len(), CUBATURE_POINT_COUNT);
    }

    #[test]
    fn wrapped_rotation_residual_uses_shortest_angle() {
        let mut state = MultivariateNormalDistribution {
            mean: vector![0.0, 0.0, PI - 0.05],
            covariance: matrix![
                0.1, 0.0, 0.0;
                0.0, 0.1, 0.0;
                0.0, 0.0, 0.2
            ],
        };
        let measurement_noise = matrix![
            0.1, 0.0;
            0.0, 0.1
        ];

        state
            .update_with_1d_translation_and_rotation(
                vector![0.0, -PI + 0.05],
                measurement_noise,
                |state| vector![state.x, state.z],
            )
            .unwrap();

        let expected_angle = wrap_angle((PI - 0.05) + (0.2 / (0.2 + 0.1)) * 0.1);
        assert_relative_eq!(state.mean.z, expected_angle, epsilon = 1.0e-5);
    }

    #[test]
    fn two_dimensional_update_uses_innovation_covariance_in_posterior_covariance() {
        let mut state = MultivariateNormalDistribution {
            mean: vector![1.0, -2.0, 0.4],
            covariance: matrix![
                0.4, 0.0, 0.0;
                0.0, 0.3, 0.0;
                0.0, 0.0, 0.2
            ],
        };
        let measurement_noise = matrix![
            0.1, 0.0;
            0.0, 0.2
        ];

        state
            .update_with_2d_translation(vector![1.0, -2.0], measurement_noise, |state| {
                vector![state.x, state.y]
            })
            .unwrap();

        assert_relative_eq!(
            state.covariance,
            matrix![
                0.08, 0.0, 0.0;
                0.0, 0.12, 0.0;
                0.0, 0.0, 0.2
            ],
            epsilon = 1.0e-5
        );
    }

    #[test]
    fn repeated_predict_and_update_cycles_keep_covariance_symmetric_and_positive_definite() {
        let mut state = MultivariateNormalDistribution {
            mean: vector![0.2, -0.3, PI - 0.1],
            covariance: matrix![
                0.3, 0.02, 0.01;
                0.02, 0.25, -0.01;
                0.01, -0.01, 0.15
            ],
        };
        let process_noise = matrix![
            0.01, 0.0, 0.0;
            0.0, 0.015, 0.0;
            0.0, 0.0, 0.02
        ];
        let line_measurement_noise = matrix![
            0.08, 0.0;
            0.0, 0.05
        ];
        let translation_measurement_noise = matrix![
            0.07, 0.0;
            0.0, 0.09
        ];

        for _ in 0..20 {
            state
                .predict(
                    |state| vector![state.x + 0.01, state.y - 0.02, state.z + 0.35],
                    process_noise,
                )
                .unwrap();
            state
                .update_with_1d_translation_and_rotation(
                    vector![0.0, -PI + 0.1],
                    line_measurement_noise,
                    |state| vector![state.x, state.z],
                )
                .unwrap();
            state
                .update_with_2d_translation(
                    vector![0.0, 0.0],
                    translation_measurement_noise,
                    |state| vector![state.x, state.y],
                )
                .unwrap();

            assert_relative_eq!(
                state.covariance,
                state.covariance.transpose(),
                epsilon = 1.0e-5
            );
            assert!(sample_cubature_points(state.mean, state.covariance).is_ok());
            assert!((-PI..=PI).contains(&state.mean.z));
        }
    }
}
