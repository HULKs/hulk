#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "IMM state is wired into tracker update in a later task"
    )
)]

use std::{f32::consts::TAU, time::Duration};

use coordinate_systems::Ground;
use filtering::kalman_filter::KalmanFilter;
use linear_algebra::{IntoFramed, Isometry2, Point2, Vector2, vector};
use nalgebra::{Matrix2, Matrix2x4, Matrix4, Matrix4x2, Vector2 as NaVector2, Vector4, matrix};
use types::multivariate_normal_distribution::MultivariateNormalDistribution;

use crate::{Measurement, TrackerParameters};

#[derive(Clone, Debug)]
pub(crate) struct ImmBallState {
    static_mode: MultivariateNormalDistribution<2>,
    rolling_mode: MultivariateNormalDistribution<4>,
    static_probability: f32,
    rolling_probability: f32,
}

impl ImmBallState {
    pub(crate) fn new_static(position: Point2<Ground>, parameters: &TrackerParameters) -> Self {
        let static_mode = MultivariateNormalDistribution {
            mean: position.inner.coords,
            covariance: Matrix2::from_diagonal(&parameters.initial_position_covariance),
        };
        let rolling_mode = MultivariateNormalDistribution {
            mean: Vector4::new(position.x(), position.y(), 0.0, 0.0),
            covariance: Matrix4::from_diagonal(&Vector4::new(
                parameters.initial_position_covariance.x,
                parameters.initial_position_covariance.y,
                parameters.initial_velocity_covariance.x,
                parameters.initial_velocity_covariance.y,
            )),
        };
        Self {
            static_mode,
            rolling_mode,
            static_probability: 0.9,
            rolling_probability: 0.1,
        }
    }

    pub(crate) fn new_rolling(
        position: Point2<Ground>,
        velocity: NaVector2<f32>,
        parameters: &TrackerParameters,
    ) -> Self {
        let mut state = Self::new_static(position, parameters);
        state.rolling_mode.mean.z = velocity.x;
        state.rolling_mode.mean.w = velocity.y;
        state.static_probability = 0.01;
        state.rolling_probability = 0.99;
        state
    }

    pub(crate) fn predict(
        &mut self,
        dt: Duration,
        odometry_delta: Isometry2<Ground, Ground>,
        parameters: &TrackerParameters,
    ) {
        self.mix_modes(parameters);
        predict_static(
            &mut self.static_mode,
            odometry_delta,
            parameters.static_process_noise,
        );
        predict_rolling(
            &mut self.rolling_mode,
            dt,
            odometry_delta,
            parameters.rolling_velocity_decay,
            parameters.rolling_process_noise,
        );
    }

    pub(crate) fn update(&mut self, measurement: &Measurement) {
        KalmanFilter::update(
            &mut self.static_mode,
            Matrix2::identity(),
            measurement.position.inner.coords,
            measurement.covariance,
        );
        KalmanFilter::update(
            &mut self.rolling_mode,
            Matrix2x4::identity(),
            measurement.position.inner.coords,
            measurement.covariance,
        );
        let static_likelihood = gaussian_log_likelihood(
            measurement.position.inner.coords,
            self.static_mode.mean,
            self.static_mode.covariance + measurement.covariance,
        );
        let rolling_likelihood = gaussian_log_likelihood(
            measurement.position.inner.coords,
            self.rolling_mode.mean.xy(),
            self.rolling_mode
                .covariance
                .fixed_view::<2, 2>(0, 0)
                .into_owned()
                + measurement.covariance,
        );
        self.reweight_modes(static_likelihood, rolling_likelihood);
    }

    pub(crate) fn position(&self) -> Point2<Ground> {
        let mean = self.static_probability * self.static_mode.mean
            + self.rolling_probability * self.rolling_mode.mean.xy();
        mean.framed().as_point()
    }

    pub(crate) fn velocity(&self) -> Vector2<Ground> {
        vector![
            self.rolling_probability * self.rolling_mode.mean.z,
            self.rolling_probability * self.rolling_mode.mean.w
        ]
    }

    pub(crate) fn position_covariance(&self) -> Matrix2<f32> {
        self.static_probability * self.static_mode.covariance
            + self.rolling_probability
                * self
                    .rolling_mode
                    .covariance
                    .fixed_view::<2, 2>(0, 0)
                    .into_owned()
    }

    pub(crate) fn measurement_log_likelihood(&self, measurement: &Measurement) -> f32 {
        gaussian_log_likelihood(
            measurement.position.inner.coords,
            self.position().inner.coords,
            self.position_covariance() + measurement.covariance,
        )
    }

    fn mix_modes(&mut self, parameters: &TrackerParameters) {
        let static_to_rolling = parameters.mode_transition_static_to_rolling;
        let rolling_to_static = parameters.mode_transition_rolling_to_static;
        let next_static = self.static_probability * (1.0 - static_to_rolling)
            + self.rolling_probability * rolling_to_static;
        let next_rolling = self.static_probability * static_to_rolling
            + self.rolling_probability * (1.0 - rolling_to_static);
        let total = next_static + next_rolling;
        self.static_probability = next_static / total;
        self.rolling_probability = next_rolling / total;
    }

    fn reweight_modes(&mut self, static_log_likelihood: f32, rolling_log_likelihood: f32) {
        let max_log_likelihood = static_log_likelihood.max(rolling_log_likelihood);
        let static_weight =
            self.static_probability * (static_log_likelihood - max_log_likelihood).exp();
        let rolling_weight =
            self.rolling_probability * (rolling_log_likelihood - max_log_likelihood).exp();
        let total = static_weight + rolling_weight;
        self.static_probability = static_weight / total;
        self.rolling_probability = rolling_weight / total;
    }
}

fn predict_static(
    state: &mut MultivariateNormalDistribution<2>,
    odometry_delta: Isometry2<Ground, Ground>,
    process_noise: Matrix2<f32>,
) {
    let rotation = odometry_delta.inner.rotation.to_rotation_matrix();
    let translation = odometry_delta.inner.translation.vector;
    KalmanFilter::predict(
        state,
        *rotation.matrix(),
        Matrix2::identity(),
        translation,
        process_noise,
    );
}

fn predict_rolling(
    state: &mut MultivariateNormalDistribution<4>,
    dt: Duration,
    odometry_delta: Isometry2<Ground, Ground>,
    velocity_decay: f32,
    process_noise: Matrix4<f32>,
) {
    let dt = dt.as_secs_f32();
    let prediction = matrix![
        1.0, 0.0, dt, 0.0;
        0.0, 1.0, 0.0, dt;
        0.0, 0.0, velocity_decay.powf(dt), 0.0;
        0.0, 0.0, 0.0, velocity_decay.powf(dt);
    ];
    let rotation = odometry_delta.inner.rotation.to_rotation_matrix();
    let rotation = rotation.matrix();
    let state_rotation = matrix![
        rotation.m11, rotation.m12, 0.0, 0.0;
        rotation.m21, rotation.m22, 0.0, 0.0;
        0.0, 0.0, rotation.m11, rotation.m12;
        0.0, 0.0, rotation.m21, rotation.m22;
    ];
    let translation = odometry_delta.inner.translation.vector;
    KalmanFilter::predict(
        state,
        prediction * state_rotation,
        Matrix4x2::identity(),
        translation,
        process_noise,
    );
}

fn gaussian_log_likelihood(
    measurement: NaVector2<f32>,
    mean: NaVector2<f32>,
    covariance: Matrix2<f32>,
) -> f32 {
    let residual = measurement - mean;
    let Some(cholesky) = covariance.cholesky() else {
        return f32::NEG_INFINITY;
    };
    let solved = cholesky.solve(&residual);
    let mahalanobis = residual.dot(&solved);
    let determinant = covariance.determinant().max(f32::MIN_POSITIVE);
    -0.5 * (mahalanobis + (TAU * determinant.sqrt()).ln())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use linear_algebra::{Isometry2, point};
    use nalgebra::Matrix2;

    use crate::{Measurement, TrackerParameters};

    use super::ImmBallState;

    fn measurement_at(x: f32, y: f32) -> Measurement {
        Measurement {
            position: point![x, y],
            covariance: Matrix2::identity() * 0.01,
            confidence: 1.0,
            image_location: None,
        }
    }

    #[test]
    fn rolling_prediction_moves_with_velocity_and_damping() {
        let parameters = TrackerParameters::default();
        let mut state =
            ImmBallState::new_rolling(point![0.0, 0.0], nalgebra::vector![1.0, 0.0], &parameters);

        state.predict(Duration::from_secs(1), Isometry2::identity(), &parameters);

        assert!(state.position().x() > 0.9);
        assert!(state.velocity().x() < 1.0);
        assert!(state.velocity().x() > 0.9);
    }

    #[test]
    fn static_update_keeps_velocity_small() {
        let parameters = TrackerParameters::default();
        let mut state = ImmBallState::new_static(point![1.0, 0.0], &parameters);

        state.update(&measurement_at(1.05, 0.0));

        assert!(state.velocity().norm() < 0.05);
    }

    #[test]
    fn measurement_log_likelihood_is_higher_near_state() {
        let parameters = TrackerParameters::default();
        let state = ImmBallState::new_static(point![0.0, 0.0], &parameters);

        let near = state.measurement_log_likelihood(&measurement_at(0.05, 0.0));
        let far = state.measurement_log_likelihood(&measurement_at(2.0, 0.0));

        assert!(near > far);
    }
}
