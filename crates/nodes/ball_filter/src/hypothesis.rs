use std::{f32::consts::PI, time::Duration};

use filtering::kalman_filter::KalmanFilter;
use moving::{MovingPredict, MovingUpdate};
use nalgebra::{Matrix2, Matrix4};
use resting::{RestingPredict, RestingUpdate};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::{IntoFramed, Isometry2, Vector2, vector};

use types::{
    ball_position::BallPosition, multivariate_normal_distribution::MultivariateNormalDistribution,
};

mod motion_evidence;
pub use motion_evidence::MotionEvidence;

pub mod moving;
pub mod resting;

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub enum BallMode {
    Resting(MultivariateNormalDistribution<2>),
    Moving(MultivariateNormalDistribution<4>),
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct BallHypothesis {
    pub mode: BallMode,
    pub last_seen: Time,
    pub validity: f32,
    /// Explicit diagnostic state, shared by live filtering and input replay.
    #[serde(default)]
    pub motion_evidence: Option<MotionEvidence>,
}

impl BallHypothesis {
    pub fn new(hypothesis: MultivariateNormalDistribution<4>, last_seen: Time) -> Self {
        Self {
            mode: BallMode::Moving(hypothesis),
            last_seen,
            validity: 1.0,
            motion_evidence: None,
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
        log_likelihood_of_zero_velocity_threshold: f32,
    ) {
        match &mut self.mode {
            BallMode::Resting(resting) => {
                if let Some(evidence) = &mut self.motion_evidence {
                    evidence.transform(last_to_current_odometry);
                }
                RestingPredict::predict(
                    resting,
                    delta_time,
                    last_to_current_odometry,
                    resting_process_noise,
                )
            }
            BallMode::Moving(moving) => {
                self.motion_evidence = None;
                MovingPredict::predict(
                    moving,
                    delta_time,
                    last_to_current_odometry,
                    velocity_decay,
                    moving_process_noise,
                );

                let velocity_covariance = moving.covariance.fixed_view::<2, 2>(2, 2);
                let velocity = nalgebra::vector![moving.mean.z, moving.mean.w];

                let log_likelihood_of_zero_velocity = if velocity_covariance == Matrix2::zeros()
                    && velocity == nalgebra::Vector2::zeros()
                {
                    // Exact zero damping is a point mass at rest, not an
                    // invertible Gaussian velocity distribution.
                    f32::INFINITY
                } else {
                    let exponent = -velocity.dot(
                        &velocity_covariance
                            .cholesky()
                            .expect("covariance not invertible")
                            .solve(&velocity),
                    ) / 2.;
                    let determinant = velocity_covariance.determinant();
                    exponent - (2. * PI * determinant.sqrt()).ln()
                };

                if log_likelihood_of_zero_velocity > log_likelihood_of_zero_velocity_threshold {
                    self.mode = BallMode::Resting(MultivariateNormalDistribution {
                        mean: moving.mean.xy(),
                        covariance: moving.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
                    })
                }
            }
        }
    }

    pub fn update(
        &mut self,
        detection_time: Time,
        measurement: MultivariateNormalDistribution<2>,
        validity_bonus: f32,
    ) {
        // Repeated or delayed exposures must not create extra confidence or
        // fictitious velocity from zero/negative observation intervals.
        if detection_time <= self.last_seen {
            return;
        }
        self.last_seen = detection_time;
        self.validity += validity_bonus;

        match &mut self.mode {
            BallMode::Resting(resting) => {
                let moving = self
                    .motion_evidence
                    .get_or_insert_with(MotionEvidence::default)
                    .observe(detection_time, measurement);
                if let Some(moving) = moving {
                    self.mode = BallMode::Moving(moving);
                    self.motion_evidence = None;
                } else {
                    RestingUpdate::update(resting, measurement);
                }
            }
            BallMode::Moving(moving) => {
                self.motion_evidence = None;
                MovingUpdate::update(moving, measurement);
            }
        }
    }

    pub fn merge(&mut self, other: BallHypothesis) {
        match (&mut self.mode, other.mode) {
            (BallMode::Resting(resting), BallMode::Resting(distribution)) => {
                KalmanFilter::update(
                    resting,
                    Matrix2::identity(),
                    distribution.mean,
                    distribution.covariance,
                );
                self.validity = self.validity.max(other.validity);
                self.last_seen = self.last_seen.max(other.last_seen);
                // Evidence from distinct tracks must not be concatenated.
                self.motion_evidence = None;
            }
            (BallMode::Moving(moving), BallMode::Moving(distribution)) => {
                KalmanFilter::update(
                    moving,
                    Matrix4::identity(),
                    distribution.mean,
                    distribution.covariance,
                );
                self.validity = self.validity.max(other.validity);
                self.last_seen = self.last_seen.max(other.last_seen);
                // Evidence from distinct tracks must not be concatenated.
                self.motion_evidence = None;
            }
            _ => (), // deny merge
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resting_decision_uses_velocity_uncertainty_not_position_uncertainty() {
        for (position_variance, velocity_variance, should_rest) in
            [(100.0, 0.01, true), (0.01, 100.0, false)]
        {
            let mut hypothesis = BallHypothesis::new(
                MultivariateNormalDistribution {
                    mean: nalgebra::Vector4::zeros(),
                    covariance: Matrix4::from_diagonal(&nalgebra::vector![
                        position_variance,
                        position_variance,
                        velocity_variance,
                        velocity_variance,
                    ]),
                },
                Time::zero(),
            );
            hypothesis.predict(
                Duration::ZERO,
                Isometry2::identity(),
                1.0,
                Matrix4::zeros(),
                Matrix2::zeros(),
                0.5,
            );
            assert_eq!(matches!(hypothesis.mode, BallMode::Resting(_)), should_rest);
        }
    }

    fn resting_hypothesis() -> BallHypothesis {
        BallHypothesis {
            mode: BallMode::Resting(MultivariateNormalDistribution {
                mean: nalgebra::Vector2::zeros(),
                covariance: Matrix2::identity() * 0.1,
            }),
            last_seen: Time::zero(),
            validity: 4.0,
            motion_evidence: None,
        }
    }

    fn observe(hypothesis: &mut BallHypothesis, millis: i64, x: f32, y: f32) {
        hypothesis.update(
            Time::from_nanos(millis * 1_000_000),
            MultivariateNormalDistribution {
                mean: nalgebra::vector![x, y],
                covariance: Matrix2::identity() * 1e-5,
            },
            1.0,
        );
    }

    #[test]
    fn coherent_observations_recover_motion_with_correlated_uncertainty() {
        let mut hypothesis = resting_hypothesis();
        observe(&mut hypothesis, 40, 0.04, 0.02);
        observe(&mut hypothesis, 80, 0.08, 0.04);
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
        observe(&mut hypothesis, 120, 0.12, 0.06);
        let BallMode::Moving(moving) = hypothesis.mode else {
            panic!("motion was not confirmed");
        };
        assert!((moving.mean.z - 1.0).abs() < 1e-5);
        assert!((moving.mean.w - 0.5).abs() < 1e-5);
        assert_eq!(moving.mean.xy(), nalgebra::vector![0.12, 0.06]);
        assert!(moving.covariance.cholesky().is_some());
        assert!(moving.covariance[(0, 2)] > 0.0);
        assert_eq!(moving.covariance[(0, 2)], moving.covariance[(2, 0)]);
        assert!(moving.covariance[(2, 2)] > 0.0);
        assert_eq!(hypothesis.validity, 7.0);
        assert_eq!(hypothesis.last_seen, Time::from_nanos(120_000_000));
        assert!(hypothesis.motion_evidence.is_none());
    }

    #[test]
    fn isolated_false_observation_does_not_confirm_motion() {
        for false_index in 0..5 {
            let mut hypothesis = resting_hypothesis();
            for index in 0..7 {
                let x = if index == false_index {
                    0.3
                } else {
                    (index % 2) as f32 * 0.001
                };
                observe(&mut hypothesis, 40 * (index + 1), x, 0.0);
                assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
            }
        }
    }

    #[test]
    fn inconsistent_motion_directions_and_speeds_do_not_confirm_motion() {
        for positions in [[0.0, 0.05, 0.01], [0.0, 0.03, 0.3]] {
            let mut hypothesis = resting_hypothesis();
            for (index, x) in positions.into_iter().enumerate() {
                observe(&mut hypothesis, 40 * (index as i64 + 1), x, 0.0);
            }
            assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
        }
    }

    #[test]
    fn repeated_or_out_of_order_observations_do_not_add_evidence_or_validity() {
        let mut hypothesis = resting_hypothesis();
        observe(&mut hypothesis, 40, 0.04, 0.0);
        observe(&mut hypothesis, 80, 0.08, 0.0);
        let position = hypothesis.position().position;
        observe(&mut hypothesis, 80, 1.0, 1.0);
        observe(&mut hypothesis, 60, -1.0, -1.0);
        assert_eq!(hypothesis.validity, 6.0);
        assert_eq!(hypothesis.position().position, position);
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
        observe(&mut hypothesis, 120, 0.12, 0.0);
        assert!((hypothesis.position().velocity.x() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn occlusion_gap_requires_new_evidence_and_does_not_average_across_the_kick() {
        let mut hypothesis = resting_hypothesis();
        observe(&mut hypothesis, 40, 0.04, 0.0);
        observe(&mut hypothesis, 80, 0.08, 0.0);
        observe(&mut hypothesis, 400, 1.0, 0.0);
        observe(&mut hypothesis, 440, 1.08, 0.0);
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
        observe(&mut hypothesis, 480, 1.16, 0.0);
        assert!((hypothesis.position().velocity.x() - 2.0).abs() < 1e-5);
    }

    #[test]
    fn stationary_noisy_ball_stays_resting_while_robot_translates_and_rotates() {
        use coordinate_systems::Odometry;
        use linear_algebra::{Pose2, point};
        let mut hypothesis = resting_hypothesis();
        let world_ball = point![1.0, 2.0];
        let mut previous_pose = Pose2::<Odometry>::new(point![0.0, 0.0], 0.0);
        for index in 1..=40 {
            let pose = Pose2::<Odometry>::new(
                point![index as f32 * 0.03, index as f32 * -0.01],
                index as f32 * 0.08,
            );
            hypothesis.predict(
                Duration::from_millis(40),
                types::odometry::previous_to_current(previous_pose, pose),
                1.0,
                Matrix4::zeros(),
                Matrix2::zeros(),
                0.5,
            );
            let in_ground = pose.as_transform::<Ground>().inverse() * world_ball;
            let noise = if index % 2 == 0 { 0.001 } else { -0.001 };
            observe(
                &mut hypothesis,
                index * 40,
                in_ground.x() + noise,
                in_ground.y() - noise,
            );
            assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
            previous_pose = pose;
        }
    }

    #[test]
    fn merging_resets_motion_evidence() {
        let mut hypothesis = resting_hypothesis();
        observe(&mut hypothesis, 40, 0.04, 0.0);
        observe(&mut hypothesis, 80, 0.08, 0.0);
        assert!(hypothesis.motion_evidence.is_some());
        hypothesis.merge(resting_hypothesis());
        assert!(hypothesis.motion_evidence.is_none());
        observe(&mut hypothesis, 120, 0.12, 0.0);
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
    }

    #[test]
    fn zero_damping_transitions_to_rest_without_inverting_singular_covariance() {
        let mut hypothesis = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 0.0, 2.0, 0.0],
                covariance: Matrix4::identity(),
            },
            Time::zero(),
        );
        hypothesis.predict(
            Duration::from_millis(2),
            Isometry2::identity(),
            0.0,
            Matrix4::zeros(),
            Matrix2::zeros(),
            0.5,
        );
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
        assert_eq!(hypothesis.position().velocity, Vector2::zeros());
    }

    #[test]
    fn closely_spaced_observations_do_not_count_as_independent_motion_evidence() {
        let mut hypothesis = resting_hypothesis();
        for millis in 1..=5 {
            observe(&mut hypothesis, millis, millis as f32 * 0.01, 0.0);
        }
        assert!(matches!(hypothesis.mode, BallMode::Resting(_)));
    }

    #[test]
    fn recovered_velocity_is_expressed_in_the_current_rotating_ground_frame() {
        use coordinate_systems::Odometry;
        use linear_algebra::{Pose2, point};
        let mut hypothesis = resting_hypothesis();
        let mut previous_pose = Pose2::<Odometry>::new(point![0.0, 0.0], 0.0);
        for index in 1..=3 {
            let elapsed = index as f32 * 0.04;
            let pose = Pose2::<Odometry>::new(
                point![index as f32 * 0.03, index as f32 * -0.01],
                index as f32 * 0.08,
            );
            let old_to_current = types::odometry::previous_to_current(previous_pose, pose);
            hypothesis.predict(
                Duration::from_millis(40),
                old_to_current,
                1.0,
                Matrix4::zeros(),
                Matrix2::zeros(),
                0.5,
            );
            let world_ball = point![1.0 + elapsed, 2.0 + 0.5 * elapsed];
            let in_ground = pose.as_transform::<Ground>().inverse() * world_ball;
            observe(&mut hypothesis, index * 40, in_ground.x(), in_ground.y());
            previous_pose = pose;
        }
        let expected_velocity =
            previous_pose.as_transform::<Ground>().inverse() * vector![<Odometry>, 1.0, 0.5];
        assert!(matches!(hypothesis.mode, BallMode::Moving(_)));
        assert!((hypothesis.position().velocity - expected_velocity).norm() < 1e-4);
    }

    #[test]
    fn successful_merges_preserve_newest_observation_in_either_order() {
        for moving in [false, true] {
            for reverse in [false, true] {
                let mut earlier = if moving {
                    BallHypothesis::new(
                        MultivariateNormalDistribution {
                            mean: nalgebra::Vector4::zeros(),
                            covariance: Matrix4::identity(),
                        },
                        Time::zero(),
                    )
                } else {
                    resting_hypothesis()
                };
                earlier.last_seen = Time::from_nanos(40_000_000);
                earlier.validity = 10.0;
                let mut later = earlier.clone();
                later.last_seen = Time::from_nanos(80_000_000);
                later.validity = 2.0;
                let (mut survivor, removed) = if reverse {
                    (later, earlier)
                } else {
                    (earlier, later)
                };
                survivor.merge(removed);
                assert_eq!(survivor.last_seen, Time::from_nanos(80_000_000));
                assert_eq!(survivor.validity, 10.0);
            }
        }
    }

    #[test]
    fn rejected_cross_mode_merge_preserves_state_and_pending_motion_evidence() {
        let mut resting = resting_hypothesis();
        observe(&mut resting, 40, 0.04, 0.0);
        observe(&mut resting, 80, 0.08, 0.0);
        let before = resting.position();
        let before_covariance = resting.position_covariance();
        let moving = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 2.0, 3.0, 4.0],
                covariance: Matrix4::identity(),
            },
            Time::from_nanos(1_000_000_000),
        );
        resting.merge(moving.clone());
        assert_eq!(resting.position().position, before.position);
        assert_eq!(resting.position().velocity, before.velocity);
        assert_eq!(resting.position_covariance(), before_covariance);
        assert_eq!(resting.last_seen, before.last_seen);
        assert_eq!(resting.validity, 6.0);
        assert!(resting.motion_evidence.is_some());
        let mut reversed = moving.clone();
        reversed.merge(resting.clone());
        assert_eq!(reversed.position().position, moving.position().position);
        assert_eq!(reversed.position().velocity, moving.position().velocity);
        assert_eq!(reversed.position_covariance(), moving.position_covariance());
        assert_eq!(reversed.last_seen, moving.last_seen);
        assert_eq!(reversed.validity, moving.validity);
        observe(&mut resting, 120, 0.12, 0.0);
        assert!((resting.position().velocity.x() - 1.0).abs() < 1e-5);
    }
}
