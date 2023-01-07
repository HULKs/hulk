use nalgebra::{SMatrix, SVector};
use types::kalman_filter::KalmanFilterSnapshot;

#[derive(Clone, Debug)]
pub struct KalmanFilter<const STATE_DIMENSION: usize> {
    state: SVector<f32, STATE_DIMENSION>,
    covariance: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
}

impl<const STATE_DIMENSION: usize> KalmanFilter<STATE_DIMENSION> {
    pub fn new(
        initial_state: SVector<f32, STATE_DIMENSION>,
        initial_covariance: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    ) -> Self {
        Self {
            state: initial_state,
            covariance: initial_covariance,
        }
    }

    pub fn predict<const CONTROL_DIMENSION: usize>(
        &mut self,
        state_prediction: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
        control_input_model: SMatrix<f32, STATE_DIMENSION, CONTROL_DIMENSION>,
        control: SVector<f32, CONTROL_DIMENSION>,
        process_noise: SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION>,
    ) {
        self.state = state_prediction * self.state + control_input_model * control;
        self.covariance =
            state_prediction * self.covariance * state_prediction.transpose() + process_noise;
    }

    pub fn update<const MEASUREMENT_DIMENSION: usize>(
        &mut self,
        measurement_prediction: SMatrix<f32, MEASUREMENT_DIMENSION, STATE_DIMENSION>,
        measurement: SVector<f32, MEASUREMENT_DIMENSION>,
        measurement_noise: SMatrix<f32, MEASUREMENT_DIMENSION, MEASUREMENT_DIMENSION>,
    ) {
        let residual = measurement - measurement_prediction * self.state;
        let residual_covariance =
            measurement_prediction * self.covariance * measurement_prediction.transpose()
                + measurement_noise;
        let kalman_gain = self.covariance
            * measurement_prediction.transpose()
            * residual_covariance
                .try_inverse()
                .expect("Residual covariance matrix is not invertible");
        self.state += kalman_gain * residual;
        self.covariance -= kalman_gain * measurement_prediction * self.covariance;
    }

    pub fn state(&self) -> SVector<f32, STATE_DIMENSION> {
        self.state
    }

    pub fn covariance(&self) -> SMatrix<f32, STATE_DIMENSION, STATE_DIMENSION> {
        self.covariance
    }
}

impl<const STATE_DIMENSION: usize> Into<KalmanFilterSnapshot<STATE_DIMENSION>>
    for KalmanFilter<STATE_DIMENSION>
{
    fn into(self) -> KalmanFilterSnapshot<STATE_DIMENSION> {
        KalmanFilterSnapshot {
            state: self.state,
            covariance: self.covariance,
        }
    }
}
