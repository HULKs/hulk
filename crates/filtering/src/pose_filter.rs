use nalgebra::{
    vector, Complex, ComplexField, Isometry2, Matrix2, Matrix3, Matrix3x2, UnitComplex, Vector2,
    Vector3,
};
use thiserror::Error;
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

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

impl PoseFilter for MultivariateNormalDistribution<3> {
    fn predict<StatePredictionFunction>(
        &mut self,
        state_prediction_function: StatePredictionFunction,
        process_noise: Matrix3<f32>,
    ) -> Result<(), Error>
    where
        StatePredictionFunction: Fn(Vector3<f32>) -> Vector3<f32>,
    {
        let sigma_points = sample_sigma_points(self.mean, self.covariance)?;
        let predicted_sigma_points: Vec<_> = sigma_points
            .iter()
            .copied()
            .map(state_prediction_function)
            .collect();
        let state_mean = mean_from_3d_sigma_points(&predicted_sigma_points);
        let state_covariance = covariance_from_3d_sigma_points(state_mean, &predicted_sigma_points);
        self.mean = state_mean;
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
        let sigma_points = sample_sigma_points(self.mean, self.covariance)?;
        let predicted_measurements: Vec<_> = sigma_points
            .iter()
            .copied()
            .map(measurement_prediction_function)
            .collect();
        let predicted_measurement_mean =
            mean_from_1d_translation_and_rotation_sigma_points(&predicted_measurements);
        let predicted_measurement_covariance =
            covariance_from_1d_translation_and_rotation_sigma_points(
                predicted_measurement_mean,
                &predicted_measurements,
            );

        let predicted_measurements_cross_covariance =
            cross_covariance_from_1d_translation_and_rotation_sigma_points(
                self.mean,
                &sigma_points,
                &predicted_measurement_mean,
                &predicted_measurements,
            );
        let kalman_gain = predicted_measurements_cross_covariance
            * (predicted_measurement_covariance + measurement_noise)
                .try_inverse()
                .ok_or(Error::Inverse)?;

        let residuum = measurement - predicted_measurement_mean;
        self.mean += kalman_gain * residuum;
        let updated_state_covariance = self.covariance
            - kalman_gain * predicted_measurement_covariance * kalman_gain.transpose();
        self.covariance = into_symmetric(updated_state_covariance);

        Ok(())
    }

    // TODO: reduce code duplication
    fn update_with_2d_translation<MeasurementPredictionFunction>(
        &mut self,
        measurement: Vector2<f32>,
        measurement_noise: Matrix2<f32>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<(), Error>
    where
        MeasurementPredictionFunction: Fn(Vector3<f32>) -> Vector2<f32>,
    {
        let sigma_points = sample_sigma_points(self.mean, self.covariance)?;
        let predicted_measurements: Vec<_> = sigma_points
            .iter()
            .copied()
            .map(measurement_prediction_function)
            .collect();
        let predicted_measurement_mean =
            mean_from_2d_translation_sigma_points(&predicted_measurements);
        let predicted_measurement_covariance = covariance_from_2d_translation_sigma_points(
            predicted_measurement_mean,
            &predicted_measurements,
        );

        let predicted_measurements_cross_covariance =
            cross_covariance_from_2d_translation_sigma_points(
                self.mean,
                &sigma_points,
                &predicted_measurement_mean,
                &predicted_measurements,
            );
        let kalman_gain = predicted_measurements_cross_covariance
            * (predicted_measurement_covariance + measurement_noise)
                .try_inverse()
                .ok_or(Error::Inverse)?;

        let residuum = measurement - predicted_measurement_mean;
        self.mean += kalman_gain * residuum;
        let updated_state_covariance = self.covariance
            - kalman_gain * predicted_measurement_covariance * kalman_gain.transpose();
        self.covariance = into_symmetric(updated_state_covariance);

        Ok(())
    }

    fn as_isometry(&self) -> Isometry2<f32> {
        Isometry2::new(vector![self.mean.x, self.mean.y], self.mean.z)
    }
}

fn into_symmetric(matrix: Matrix3<f32>) -> Matrix3<f32> {
    0.5 * (matrix + matrix.transpose())
}

fn sample_sigma_points(
    mean: Vector3<f32>,
    covariance: Matrix3<f32>,
) -> Result<[Vector3<f32>; 7], Error> {
    let covariance_cholesky = covariance.cholesky().ok_or(Error::Cholesky)?;
    let covariance_square_root = covariance_cholesky.l();

    let sigma_points = [
        mean,
        mean + covariance_square_root.column(0),
        mean - covariance_square_root.column(0),
        mean + covariance_square_root.column(1),
        mean - covariance_square_root.column(1),
        mean + covariance_square_root.column(2),
        mean - covariance_square_root.column(2),
    ];
    Ok(sigma_points)
}

fn mean_from_3d_sigma_points(points: &[Vector3<f32>]) -> Vector3<f32> {
    let mut mean = Vector2::zeros();
    let mut mean_angle = Complex::new(0.0, 0.0);
    for point in points {
        mean += point.xy();
        mean_angle += Complex::new(point.z.cos(), point.z.sin());
    }
    mean *= 1.0 / 7.0;
    vector![mean.x, mean.y, mean_angle.argument()]
}

fn mean_from_1d_translation_and_rotation_sigma_points(points: &[Vector2<f32>]) -> Vector2<f32> {
    let mut mean_x = 0.0;
    let mut mean_angle = Complex::new(0.0, 0.0);
    for point in points {
        mean_x += point.x;
        mean_angle += Complex::new(point.y.cos(), point.y.sin());
    }
    mean_x *= 1.0 / 7.0;
    vector![mean_x, mean_angle.argument()]
}

fn mean_from_2d_translation_sigma_points(points: &[Vector2<f32>]) -> Vector2<f32> {
    let mut mean = Vector2::zeros();
    for point in points {
        mean += point;
    }
    mean *= 1.0 / 7.0;
    mean
}

fn covariance_from_3d_sigma_points(
    mean: Vector3<f32>,
    sigma_points: &[Vector3<f32>],
) -> Matrix3<f32> {
    sigma_points
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
        * (1.0 / 6.0)
}

fn covariance_from_1d_translation_and_rotation_sigma_points(
    mean: Vector2<f32>,
    sigma_points: &[Vector2<f32>],
) -> Matrix2<f32> {
    sigma_points
        .iter()
        .map(|point| {
            vector![
                point.x - mean.x,
                (UnitComplex::new(point.y) / UnitComplex::new(mean.y)).angle()
            ]
        })
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<Matrix2<f32>>()
        * (1.0 / 6.0)
}

fn covariance_from_2d_translation_sigma_points(
    mean: Vector2<f32>,
    sigma_points: &[Vector2<f32>],
) -> Matrix2<f32> {
    sigma_points
        .iter()
        .map(|point| point - mean)
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<Matrix2<f32>>()
        * (1.0 / 6.0)
}

fn cross_covariance_from_1d_translation_and_rotation_sigma_points(
    state_mean: Vector3<f32>,
    state_sigma_points: &[Vector3<f32>],
    &measurement_mean: &Vector2<f32>,
    measurement_sigma_points: &[Vector2<f32>],
) -> Matrix3x2<f32> {
    assert!(state_sigma_points.len() == measurement_sigma_points.len());
    state_sigma_points
        .iter()
        .zip(measurement_sigma_points.iter())
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
        * (1.0 / 6.0)
}

fn cross_covariance_from_2d_translation_sigma_points(
    state_mean: Vector3<f32>,
    state_sigma_points: &[Vector3<f32>],
    &measurement_mean: &Vector2<f32>,
    measurement_sigma_points: &[Vector2<f32>],
) -> Matrix3x2<f32> {
    assert!(state_sigma_points.len() == measurement_sigma_points.len());
    state_sigma_points
        .iter()
        .zip(measurement_sigma_points.iter())
        .map(|(state, measurement)| {
            vector![
                state.x - state_mean.x,
                state.y - state_mean.y,
                (UnitComplex::new(state.z) / UnitComplex::new(state_mean.z)).angle()
            ] * (measurement - measurement_mean).transpose()
        })
        .sum::<Matrix3x2<f32>>()
        * (1.0 / 6.0)
}
