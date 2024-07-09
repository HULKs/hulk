use std::time::{Duration, SystemTime};

use filtering::kalman_filter::KalmanFilter;
use moving::{MovingPredict, MovingUpdate};
use nalgebra::{Matrix2, Matrix4};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use resting::{RestingPredict, RestingUpdate};
use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{vector, IntoFramed, Isometry2, Point2, Vector2};

use types::{
    ball_position::BallPosition, multivariate_normal_distribution::MultivariateNormalDistribution,
};

pub mod moving;
pub mod resting;

#[derive(Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect)]
pub struct BallHypothesis {
    moving: MultivariateNormalDistribution<4>,
    resting: MultivariateNormalDistribution<2>,
    last_seen: SystemTime,
    pub validity: f32,
}

impl BallHypothesis {
    pub fn new(
        moving_hypothesis: MultivariateNormalDistribution<4>,
        resting_hypothesis: MultivariateNormalDistribution<2>,
        last_seen: SystemTime,
    ) -> Self {
        Self {
            moving: moving_hypothesis,
            resting: resting_hypothesis,
            last_seen,
            validity: 1.0,
        }
    }

    pub fn resting(&self) -> BallPosition<Ground> {
        BallPosition {
            position: self.resting.mean.xy().framed().as_point(),
            velocity: Vector2::zeros(),
            last_seen: self.last_seen,
        }
    }

    pub fn moving(&self) -> BallPosition<Ground> {
        BallPosition {
            position: self.moving.mean.xy().framed().as_point(),
            velocity: vector![self.moving.mean.z, self.moving.mean.w],
            last_seen: self.last_seen,
        }
    }

    pub fn choose_ball(&self, velocity_threshold: f32) -> BallPosition<Ground> {
        let moving = self.moving();
        if moving.velocity.norm() < velocity_threshold {
            return self.resting();
        };
        moving
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
        MovingPredict::predict(
            &mut self.moving,
            delta_time,
            last_to_current_odometry,
            velocity_decay,
            moving_process_noise,
        );
        RestingPredict::predict(
            &mut self.resting,
            last_to_current_odometry,
            resting_process_noise,
        );

        let moving_velocity: Vector2<Ground> = vector![self.moving.mean.z, self.moving.mean.w];
        if moving_velocity.norm() < velocity_threshold {
            self.resting.mean.x = self.moving.mean.x;
            self.resting.mean.y = self.moving.mean.y;
        }
    }

    pub fn update(
        &mut self,
        detection_time: SystemTime,
        measurement: Point2<Ground>,
        noise: Matrix2<f32>,
    ) {
        self.last_seen = detection_time;
        MovingUpdate::update(&mut self.moving, measurement, noise);
        RestingUpdate::update(&mut self.resting, measurement, noise);
        self.validity += 1.0;
    }

    pub fn merge(&mut self, other: BallHypothesis) {
        KalmanFilter::update(
            &mut self.moving,
            Matrix4::identity(),
            other.moving.mean,
            other.moving.covariance,
        );
        KalmanFilter::update(
            &mut self.resting,
            Matrix2::identity(),
            other.resting.mean,
            other.resting.covariance,
        );
    }
}
