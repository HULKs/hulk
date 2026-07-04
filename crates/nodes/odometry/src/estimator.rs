use booster::{FallDownState, FallDownStateType, ImuState};
use coordinate_systems::{Ground, LeftSole, Odometry, RightSole, Robot};
use kinematics::{
    robot_kinematics::RobotKinematics,
    sole_contact::{SoleSide, SupportSelectionParameters},
};
use linear_algebra::{Isometry2, Point3, Pose2, Vector2};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};

/// Parameters controlling contact-aware odometry integration.
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    /// Maximum vertical distance between sole vertices that are treated as simultaneous ground
    /// contacts.
    pub contact_height_epsilon: f32,
    /// Maximum absolute contact-height difference for treating both soles as double support.
    pub double_support_deadband: f32,
    /// Extra contact-height margin required before switching support to the other sole.
    pub support_switch_hysteresis: f32,
    /// Per-axis multiplier applied to estimated ground translation.
    pub translation_scale: Vector2<Robot>,
    /// Maximum accepted linear speed for one odometry update in meters per second.
    pub max_linear_speed: f32,
    /// Maximum accepted angular speed for one odometry update in radians per second.
    pub max_angular_speed: f32,
    /// Candidate left-sole vertices used to estimate the current contact point.
    pub left_sole_contact_vertices: Vec<Point3<LeftSole>>,
    /// Candidate right-sole vertices used to estimate the current contact point.
    pub right_sole_contact_vertices: Vec<Point3<RightSole>>,
}

pub struct EstimatorInput<'a> {
    pub time: Time,
    pub imu_state: &'a ImuState,
    pub robot_kinematics: Option<&'a RobotKinematics>,
    pub fall_down_state: Option<&'a FallDownState>,
}

#[derive(Debug, Default)]
pub struct OdometryEstimator {
    pose: Pose2<Odometry>,
    yaw_offset_at_start: Option<f32>,
    last_yaw: Option<f32>,
    last_time: Option<Time>,
    support_side: Option<SoleSide>,
    previous_support_contact: Option<Vector2<Robot>>,
}

impl OdometryEstimator {
    pub fn update(
        &mut self,
        input: EstimatorInput<'_>,
        parameters: &Parameters,
    ) -> Option<Pose2<Odometry>> {
        let robot_kinematics = input.robot_kinematics?;

        let yaw_offset = *self
            .yaw_offset_at_start
            .get_or_insert(input.imu_state.roll_pitch_yaw.z());
        let yaw = normalize_angle(input.imu_state.roll_pitch_yaw.z() - yaw_offset);

        if !is_ready(input.fall_down_state) {
            self.clear_contact_tracking();
            self.pose = pose_with_yaw(self.pose, yaw);
            self.last_time = Some(input.time);
            return Some(self.pose);
        }

        let left_contact = kinematics::sole_contact::estimate_sole_contact(
            robot_kinematics.left_leg.sole_to_robot,
            input.imu_state.roll_pitch_yaw.x(),
            input.imu_state.roll_pitch_yaw.y(),
            &parameters.left_sole_contact_vertices,
            parameters.contact_height_epsilon,
        )?;
        let right_contact = kinematics::sole_contact::estimate_sole_contact(
            robot_kinematics.right_leg.sole_to_robot,
            input.imu_state.roll_pitch_yaw.x(),
            input.imu_state.roll_pitch_yaw.y(),
            &parameters.right_sole_contact_vertices,
            parameters.contact_height_epsilon,
        )?;
        let support_side = kinematics::sole_contact::select_support_side(
            left_contact,
            right_contact,
            self.support_side,
            SupportSelectionParameters {
                double_support_deadband: parameters.double_support_deadband,
                support_switch_hysteresis: parameters.support_switch_hysteresis,
            },
        );

        let Some(support_side) = support_side else {
            self.clear_contact_tracking();
            self.pose = pose_with_yaw(self.pose, yaw);
            self.last_time = Some(input.time);
            return Some(self.pose);
        };

        let current_contact = match support_side {
            SoleSide::Left => left_contact.contact_xy_in_leveled_robot,
            SoleSide::Right => right_contact.contact_xy_in_leveled_robot,
        };

        let Some(previous_contact) = self.previous_support_contact else {
            self.anchor(support_side, current_contact, yaw, input.time);
            return Some(self.pose);
        };

        if self.support_side != Some(support_side) {
            self.anchor(support_side, current_contact, yaw, input.time);
            return Some(self.pose);
        }

        let yaw_delta = normalize_angle(yaw - self.last_yaw.unwrap_or(yaw));
        let rotation = nalgebra::Rotation2::new(yaw_delta);
        let mut translation = previous_contact - Vector2::wrap(rotation * current_contact.inner);
        translation.inner.x *= parameters.translation_scale.x();
        translation.inner.y *= parameters.translation_scale.y();

        if self.delta_exceeds_limits(input.time, translation, yaw_delta, parameters) {
            self.anchor(support_side, current_contact, yaw, input.time);
            return Some(self.pose);
        }

        let current_to_previous =
            Isometry2::<Ground, Ground>::from_parts(Vector2::wrap(translation.inner), yaw_delta);
        let previous_to_odometry = self.pose.as_transform::<Ground>();
        let current_to_odometry = previous_to_odometry * current_to_previous;
        self.pose = pose_with_yaw(current_to_odometry.as_pose(), yaw);

        self.previous_support_contact = Some(current_contact);
        self.last_yaw = Some(yaw);
        self.last_time = Some(input.time);

        Some(self.pose)
    }

    fn clear_contact_tracking(&mut self) {
        self.support_side = None;
        self.previous_support_contact = None;
        self.last_yaw = None;
    }

    fn anchor(&mut self, support_side: SoleSide, contact: Vector2<Robot>, yaw: f32, time: Time) {
        self.support_side = Some(support_side);
        self.previous_support_contact = Some(contact);
        self.last_yaw = Some(yaw);
        self.last_time = Some(time);
        self.pose = pose_with_yaw(self.pose, yaw);
    }

    fn delta_exceeds_limits(
        &self,
        time: Time,
        translation: Vector2<Robot>,
        yaw_delta: f32,
        parameters: &Parameters,
    ) -> bool {
        let Some(last_time) = self.last_time else {
            return false;
        };
        let delta_time = time.duration_since(last_time).as_secs_f32();
        if delta_time <= 0.0 {
            return false;
        }

        let linear_speed = translation.norm() / delta_time;
        let angular_speed = yaw_delta.abs() / delta_time;
        linear_speed > parameters.max_linear_speed || angular_speed > parameters.max_angular_speed
    }
}

fn pose_with_yaw(pose: Pose2<Odometry>, yaw: f32) -> Pose2<Odometry> {
    Pose2::new(pose.position(), yaw)
}

fn normalize_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

fn is_ready(fall_down_state: Option<&FallDownState>) -> bool {
    matches!(
        fall_down_state,
        Some(FallDownState {
            fall_down_state: FallDownStateType::IsReady,
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use booster::{FallDownState, FallDownStateType, ImuState};
    use coordinate_systems::{LeftSole, RightSole};
    use kinematics::robot_kinematics::{
        RobotKinematics, RobotLeftLegKinematics, RobotRightLegKinematics,
    };
    use linear_algebra::{IntoTransform, point, vector};
    use ros_z::time::Time;

    use super::*;

    fn parameters() -> Parameters {
        Parameters {
            contact_height_epsilon: 0.001,
            double_support_deadband: 0.001,
            support_switch_hysteresis: 0.002,
            translation_scale: vector![<Robot>, 1.0, 1.0],
            max_linear_speed: 10.0,
            max_angular_speed: 10.0,
            left_sole_contact_vertices: vec![
                point![<LeftSole>, 0.1, 0.05, 0.0],
                point![<LeftSole>, 0.1, -0.05, 0.0],
                point![<LeftSole>, -0.1, 0.05, 0.0],
                point![<LeftSole>, -0.1, -0.05, 0.0],
            ],
            right_sole_contact_vertices: vec![
                point![<RightSole>, 0.1, 0.05, 0.0],
                point![<RightSole>, 0.1, -0.05, 0.0],
                point![<RightSole>, -0.1, 0.05, 0.0],
                point![<RightSole>, -0.1, -0.05, 0.0],
            ],
        }
    }

    fn imu(yaw: f32) -> ImuState {
        ImuState {
            roll_pitch_yaw: vector![<Robot>, 0.0, 0.0, yaw],
            angular_velocity: vector![<Robot>, 0.0, 0.0, 0.0],
            linear_acceleration: vector![<Robot>, 0.0, 0.0, 0.0],
        }
    }

    fn ready() -> FallDownState {
        FallDownState {
            fall_down_state: FallDownStateType::IsReady,
            is_recovery_available: false,
        }
    }

    fn fallen() -> FallDownState {
        FallDownState {
            fall_down_state: FallDownStateType::HasFallen,
            is_recovery_available: true,
        }
    }

    fn kinematics(left_x: f32, right_x: f32) -> RobotKinematics {
        kinematics_with_heights(left_x, -0.02, right_x, 0.0)
    }

    fn kinematics_with_heights(
        left_x: f32,
        left_z: f32,
        right_x: f32,
        right_z: f32,
    ) -> RobotKinematics {
        RobotKinematics {
            left_leg: RobotLeftLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(left_x, 0.05, left_z)
                    .framed_transform(),
                ..Default::default()
            },
            right_leg: RobotRightLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(right_x, -0.05, right_z)
                    .framed_transform(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn first_valid_frame_anchors_without_translation() {
        let mut estimator = OdometryEstimator::default();
        let imu = imu(1.0);
        let kinematics = kinematics(0.0, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_000_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&kinematics),
                    fall_down_state: Some(&ready()),
                },
                &parameters(),
            )
            .expect("valid frame publishes");

        assert!(pose.position().x().abs() < 1.0e-6);
        assert!(pose.position().y().abs() < 1.0e-6);
        assert!(pose.orientation().angle().abs() < 1.0e-6);
    }

    #[test]
    fn support_contact_motion_integrates_opposite_robot_motion() {
        let mut estimator = OdometryEstimator::default();
        let parameters = parameters();
        let imu = imu(0.0);
        let first = kinematics(0.0, 0.0);
        estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: Some(&first),
                fall_down_state: Some(&ready()),
            },
            &parameters,
        );

        let second = kinematics(0.05, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&second),
                    fall_down_state: Some(&ready()),
                },
                &parameters,
            )
            .expect("second frame publishes");

        assert!(pose.position().x() < -0.049);
    }

    #[test]
    fn support_switch_reanchors_without_translation_jump() {
        let mut estimator = OdometryEstimator::default();
        let parameters = parameters();
        let imu = imu(0.0);
        let first = kinematics_with_heights(0.0, -0.02, 0.0, 0.0);
        estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: Some(&first),
                fall_down_state: Some(&ready()),
            },
            &parameters,
        );

        let second = kinematics_with_heights(0.5, 0.0, 0.5, -0.02);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&second),
                    fall_down_state: Some(&ready()),
                },
                &parameters,
            )
            .expect("support switch frame publishes");

        assert!(pose.position().x().abs() < 1.0e-6);
        assert!(pose.position().y().abs() < 1.0e-6);
    }

    #[test]
    fn excessive_delta_rejection_keeps_last_valid_pose_and_reanchors() {
        let mut estimator = OdometryEstimator::default();
        let mut parameters = parameters();
        parameters.max_linear_speed = 0.1;
        let imu = imu(0.0);
        let first = kinematics(0.0, 0.0);
        estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: Some(&first),
                fall_down_state: Some(&ready()),
            },
            &parameters,
        );

        let second = kinematics(1.0, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&second),
                    fall_down_state: Some(&ready()),
                },
                &parameters,
            )
            .expect("rejected-delta frame still publishes last pose");

        assert!(pose.position().x().abs() < 1.0e-6);
        assert!(pose.position().y().abs() < 1.0e-6);
    }

    #[test]
    fn falling_state_clears_anchor_and_suppresses_translation() {
        let mut estimator = OdometryEstimator::default();
        let parameters = parameters();
        let imu = imu(0.0);
        let first = kinematics(0.0, 0.0);
        estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: Some(&first),
                fall_down_state: Some(&ready()),
            },
            &parameters,
        );

        let second = kinematics(0.5, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&second),
                    fall_down_state: Some(&fallen()),
                },
                &parameters,
            )
            .expect("falling frame publishes unchanged pose");

        assert!(pose.position().x().abs() < 1.0e-6);
    }

    #[test]
    fn missing_fall_state_clears_anchor_and_suppresses_translation() {
        let mut estimator = OdometryEstimator::default();
        let parameters = parameters();
        let imu = imu(0.0);
        let first = kinematics(0.0, 0.0);
        estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: Some(&first),
                fall_down_state: Some(&ready()),
            },
            &parameters,
        );

        let second = kinematics(0.5, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &imu,
                    robot_kinematics: Some(&second),
                    fall_down_state: None,
                },
                &parameters,
            )
            .expect("missing fall state publishes unchanged pose");

        assert!(pose.position().x().abs() < 1.0e-6);
    }

    #[test]
    fn missing_kinematics_skips_publication() {
        let mut estimator = OdometryEstimator::default();
        let imu = imu(0.0);

        let pose = estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &imu,
                robot_kinematics: None,
                fall_down_state: Some(&ready()),
            },
            &parameters(),
        );

        assert!(pose.is_none());
    }

    #[test]
    fn skipped_missing_kinematics_sample_does_not_initialize_yaw_offset() {
        let mut estimator = OdometryEstimator::default();
        let skipped_imu = imu(1.0);
        let pose = estimator.update(
            EstimatorInput {
                time: Time::from_nanos(1_000_000_000),
                imu_state: &skipped_imu,
                robot_kinematics: None,
                fall_down_state: Some(&ready()),
            },
            &parameters(),
        );

        assert!(pose.is_none());

        let valid_imu = imu(1.5);
        let kinematics = kinematics(0.0, 0.0);
        let pose = estimator
            .update(
                EstimatorInput {
                    time: Time::from_nanos(1_010_000_000),
                    imu_state: &valid_imu,
                    robot_kinematics: Some(&kinematics),
                    fall_down_state: Some(&ready()),
                },
                &parameters(),
            )
            .expect("first valid frame publishes");

        assert!(pose.orientation().angle().abs() < 1.0e-6);
    }
}
