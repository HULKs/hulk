use nalgebra::{SMatrix, SVector};
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

pub trait KalmanFilter<const STATE_DIMENSION: usize> {
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
    );
}

impl<const STATE_DIMENSION: usize> KalmanFilter<STATE_DIMENSION>
    for MultivariateNormalDistribution<STATE_DIMENSION>
{
    fn predict<const CONTROL_DIMENSION: usize>(
        &mut self,
        state_prediction: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
        control_input_model: SMatrix<f32, STATE_DIMENSION, CONTROL_DIMENSION>,
        control: SVector<f32, CONTROL_DIMENSION>,
        process_noise: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    ) {
        self.mean = state_prediction * self.mean + control_input_model * control;
        self.covariance =
            state_prediction * self.covariance * state_prediction.transpose() + process_noise;
    }

    fn update<const MEASUREMENT_DIMENSION: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
    ) {
        let residual = measurement - measurement_prediction * self.mean;
        let residual_covariance =
            measurement_prediction * self.covariance * measurement_prediction.transpose()
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
