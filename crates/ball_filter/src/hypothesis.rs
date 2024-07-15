use std::time::{Duration, SystemTime};

use filtering::kalman_filter::KalmanFilter;
use moving::{MovingPredict, MovingUpdate};
use nalgebra::{Matrix2, Matrix4};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use resting::{RestingPredict, RestingUpdate};
use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{vector, IntoFramed, Isometry2, Vector2};

use types::{
    ball_position::BallPosition, multivariate_normal_distribution::MultivariateNormalDistribution,
};

pub mod moving;
pub mod resting;

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub enum BallMode {
    Resting(MultivariateNormalDistribution<2>),
    Moving(MultivariateNormalDistribution<4>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct BallHypothesis {
    pub mode: BallMode,
    pub last_seen: SystemTime,
    pub validity: f32,
}

impl BallHypothesis {
    pub fn new(hypothesis: MultivariateNormalDistribution<4>, last_seen: SystemTime) -> Self {
        Self {
            mode: BallMode::Moving(hypothesis),
            last_seen,
            validity: 1.0,
        }
    }

    pub fn position(&self) -> BallPosition<Ground> {
        match self.mode {
            BallMode::Resting(resting) => BallPosition {
                position: resting.mean.framed().as_point(),
                velocity: Vector2::zeros(),
                last_seen: self.last_seen,
            },
            BallMode::Moving(moving) => BallPosition {
                position: moving.mean.xy().framed().as_point(),
                velocity: vector![moving.mean.z, moving.mean.w],
                last_seen: self.last_seen,
            },
        }
    }

    pub fn position_covariance(&self) -> Matrix2<f32> {
        match self.mode {
            BallMode::Resting(resting) => resting.covariance,
            BallMode::Moving(moving) => moving.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
        }
    }

    pub fn predict(
        &mut self,
        delta_time: Duration,
        last_to_current_odometry: Isometry2<Ground, Ground>,
        velocity_decay: f32,
        moving_process_noise: Matrix4<f32>,
        resting_process_noise: Matrix2<f32>,
        velocity_threshold: f32,
    ) {
        match &mut self.mode {
            BallMode::Resting(resting) => {
                RestingPredict::predict(resting, last_to_current_odometry, resting_process_noise)
            }
            BallMode::Moving(moving) => {
                MovingPredict::predict(
                    moving,
                    delta_time,
                    last_to_current_odometry,
                    velocity_decay,
                    moving_process_noise,
                );
            }
        }
        if self.position().velocity.norm() < velocity_threshold {
            self.mode = BallMode::Resting(MultivariateNormalDistribution {
                mean: moving.mean.xy(),
                covariance: moving.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
            })
        }
    }

    pub fn update(
        &mut self,
        detection_time: SystemTime,
        measurement: MultivariateNormalDistribution<2>,
        validity_bonus: f32,
    ) {
        self.last_seen = detection_time;
        self.validity += validity_bonus;

        match &mut self.mode {
            BallMode::Resting(resting) => RestingUpdate::update(resting, measurement),
            BallMode::Moving(moving) => MovingUpdate::update(moving, measurement),
        }
    }

    pub fn merge(&mut self, other: BallHypothesis) {
        match (&mut self.mode, other.mode) {
            (BallMode::Resting(resting), BallMode::Resting(other)) => {
                KalmanFilter::update(resting, Matrix2::identity(), other.mean, other.covariance)
            }
            (BallMode::Moving(moving), BallMode::Moving(other)) => {
                KalmanFilter::update(moving, Matrix4::identity(), other.mean, other.covariance)
            }
            _ => (), // deny merge
        };
    }
}
