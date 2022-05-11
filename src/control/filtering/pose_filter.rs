use anyhow::{Context, Result};
use macros::SerializeHierarchy;
use nalgebra::{vector, Isometry2, SMatrix, SVector, Vector2, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct PoseFilter {
    score: f32,
    state_mean: Vector3<f32>,
    state_covariance: SMatrix<f32, 3, 3>,
}

impl PoseFilter {
    pub fn new(
        initial_state: Vector3<f32>,
        inital_state_covariance: SMatrix<f32, 3, 3>,
        initial_score: f32,
    ) -> Self {
        Self {
            score: initial_score,
            state_mean: initial_state,
            state_covariance: inital_state_covariance,
        }
    }

    pub fn add_score(&mut self, score: f32) {
        self.score += score;
    }

    pub fn score(&self) -> f32 {
        self.score
    }

    pub fn predict<StatePredictionFunction>(
        &mut self,
        state_prediction_function: StatePredictionFunction,
        process_noise: SMatrix<f32, 3, 3>,
    ) -> Result<()>
    where
        StatePredictionFunction: Fn(&Vector3<f32>) -> Vector3<f32>,
    {
        let sigma_points = sample_sigma_points(&self.state_mean, &self.state_covariance)?;
        let predicted_sigma_points: Vec<_> =
            sigma_points.iter().map(state_prediction_function).collect();
        let state_mean = mean_from_3d_sigma_points(&predicted_sigma_points);
        let state_covariance = covariance_from_sigma_points(&state_mean, &predicted_sigma_points);
        self.state_mean = state_mean;
        self.state_covariance = into_symmetric(state_covariance + process_noise);
        self.score *= 0.5;

        Ok(())
    }

    pub fn update<MeasurementPredictionFunction>(
        &mut self,
        measurement: SVector<f32, 2>,
        measurement_noise: SMatrix<f32, 2, 2>,
        measurement_prediction_function: MeasurementPredictionFunction,
    ) -> Result<()>
    where
        MeasurementPredictionFunction: Fn(&Vector3<f32>) -> Vector2<f32>,
    {
        let sigma_points = sample_sigma_points(&self.state_mean, &self.state_covariance)?;
        let predicted_measurements: Vec<_> = sigma_points
            .iter()
            .map(measurement_prediction_function)
            .collect();
        let predicted_measurement_mean = mean_from_2d_sigma_points(&predicted_measurements);
        let predicted_measurement_covariance =
            covariance_from_sigma_points(&predicted_measurement_mean, &predicted_measurements);

        let predicted_measurements_cross_covariance = cross_covariance_from_sigma_points(
            &self.state_mean,
            &sigma_points,
            &predicted_measurement_mean,
            &predicted_measurements,
        );
        let kalman_gain = predicted_measurements_cross_covariance
            * (predicted_measurement_covariance + measurement_noise)
                .try_inverse()
                .context("Failed to invert measurement covariance matrix")?;

        let residuum = measurement - predicted_measurement_mean;
        self.state_mean += kalman_gain * residuum;
        let updated_state_covariance = self.state_covariance
            - kalman_gain * predicted_measurement_covariance * kalman_gain.transpose();
        self.state_covariance = into_symmetric(updated_state_covariance);
        Ok(())
    }

    pub fn state_mean(&self) -> Vector3<f32> {
        self.state_mean
    }

    pub fn isometry(&self) -> Isometry2<f32> {
        Isometry2::new(
            vector![self.state_mean.x, self.state_mean.y],
            self.state_mean.z,
        )
    }

    pub fn state_covariance(&self) -> SMatrix<f32, 3, 3> {
        self.state_covariance
    }
}

fn into_symmetric<const DIMENSION: usize>(
    matrix: SMatrix<f32, DIMENSION, DIMENSION>,
) -> SMatrix<f32, DIMENSION, DIMENSION> {
    0.5 * (matrix + matrix.transpose())
}

fn sample_sigma_points(
    &mean: &Vector3<f32>,
    covariance: &SMatrix<f32, 3, 3>,
) -> Result<[Vector3<f32>; 7]> {
    let covariance_cholesky = covariance.cholesky().with_context(|| {
        format!(
            "Failed to decompose covariance matrix via Cholesky decomposition. Matrix was: {}",
            covariance
        )
    })?;
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

fn covariance_from_sigma_points<const DIMENSION: usize>(
    &mean: &SVector<f32, DIMENSION>,
    sigma_points: &[SVector<f32, DIMENSION>],
) -> SMatrix<f32, DIMENSION, DIMENSION> {
    sigma_points
        .iter()
        .map(|point| point - mean)
        .map(|normalized_point| normalized_point * normalized_point.transpose())
        .sum::<SMatrix<f32, DIMENSION, DIMENSION>>()
        * 0.5
}

fn cross_covariance_from_sigma_points(
    &state_mean: &Vector3<f32>,
    state_sigma_points: &[Vector3<f32>],
    &measurement_mean: &SVector<f32, 2>,
    measurement_sigma_points: &[SVector<f32, 2>],
) -> SMatrix<f32, 3, 2> {
    assert!(state_sigma_points.len() == measurement_sigma_points.len());
    state_sigma_points
        .iter()
        .zip(measurement_sigma_points.iter())
        .map(|(state, measurement)| {
            (state - state_mean) * (measurement - measurement_mean).transpose()
        })
        .sum::<SMatrix<f32, 3, 2>>()
        * 0.5
}

fn mean_from_2d_sigma_points(points: &[SVector<f32, 2>]) -> SVector<f32, 2> {
    let mut mean_x = 0.0;
    let mut mean_direction = vector![0.0, 0.0];
    for point in points {
        mean_x += point.x;
        mean_direction += vector![point.y.cos(), point.y.sin()];
    }
    mean_x *= 1.0 / 7.0;
    mean_direction *= 1.0 / 7.0;
    vector![mean_x, mean_direction.y.atan2(mean_direction.x)]
}

fn mean_from_3d_sigma_points(points: &[SVector<f32, 3>]) -> SVector<f32, 3> {
    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut mean_direction = vector![0.0, 0.0];
    for point in points {
        mean_x += point.x;
        mean_y += point.y;
        mean_direction += vector![point.z.cos(), point.z.sin()];
    }
    mean_x *= 1.0 / 7.0;
    mean_y *= 1.0 / 7.0;
    mean_direction *= 1.0 / 7.0;
    vector![mean_x, mean_y, mean_direction.y.atan2(mean_direction.x)]
}
