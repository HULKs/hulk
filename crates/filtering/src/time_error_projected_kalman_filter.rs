use nalgebra::{SMatrix, SVector};
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

use crate::kalman_filter::KalmanFilter;

pub trait TimeErrorProjectedKalmanFilter<const STATE_DIMENSION: usize> {
    fn predict<const CONTROL_DIMENSION: usize>(
        &mut self,
        state_prediction: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
        control_input_model: SMatrix<f32, STATE_DIMENSION, CONTROL_DIMENSION>,
        control: SVector<f32, CONTROL_DIMENSION>,
        process_noise: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    );
    fn update<const MEASUREMENT_DIMENSION: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
        state_derivative: SVector<f32, STATE_DIMENSION>,
        measurement_time_noise: f32,
    );
}

impl<const STATE_DIMENSION: usize> TimeErrorProjectedKalmanFilter<STATE_DIMENSION>
    for MultivariateNormalDistribution<STATE_DIMENSION>
{
    fn predict<const CONTROL_DIMENSION: usize>(
        &mut self,
        state_prediction: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
        control_input_model: SMatrix<f32, STATE_DIMENSION, CONTROL_DIMENSION>,
        control: SVector<f32, CONTROL_DIMENSION>,
        process_noise: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    ) {
        KalmanFilter::<STATE_DIMENSION>::predict(
            self,
            state_prediction,
            control_input_model,
            control,
            process_noise,
        )
    }

    fn update<const MEASUREMENT_DIMENSION: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
        state_derivative: SVector<f32, STATE_DIMENSION>,
        measurement_noise_variance: f32,
    ) {
        let residual = measurement - measurement_prediction * self.mean;
        let time_error_adjusted_covariance = self.covariance
            + measurement_noise_variance * state_derivative * state_derivative.transpose();
        let residual_covariance = measurement_prediction
            * time_error_adjusted_covariance
            * measurement_prediction.transpose()
            + measurement_noise;
        let kalman_gain = self.covariance
            * measurement_prediction.transpose()
            * residual_covariance
                .try_inverse()
                .expect("Residual covariance matrix is not invertible");
        self.mean += kalman_gain * residual;
        self.covariance -= kalman_gain * measurement_prediction * self.covariance;
    }
}
