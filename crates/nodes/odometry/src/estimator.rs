use booster::ImuState;
use coordinate_systems::{Ground, LeftSole, Odometry, RightSole, Robot};
use kinematics::{
    robot_kinematics::RobotKinematics,
    sole_contact::{SoleSide, SupportSelectionParameters},
};
use linear_algebra::{Isometry2, Point3, Pose2, Vector2};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use types::fall_detection::{FallDetection, Posture};

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
    pub fall_detection: Option<&'a FallDetection>,
}

#[derive(Debug, Default)]
pub struct OdometryEstimator {
    pose: Pose2<Odometry>,
    yaw_offset_at_start: Option<f32>,
    last_yaw: Option<f32>,
    last_time: Option<Time>,
    support_side: Option<SoleSide>,
    previous_support_contact: Option<Vector2<Robot>>,
    previous_ground_origin: Option<Vector2<Robot>>,
}

impl OdometryEstimator {
    /// Integrates the pose of Ground, the point between the feet.
    ///
    /// Contacts measure motion of the robot origin. Ground also moves relative to that origin
    /// during each step, so its change of offset must be included in the published pose.
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

        if !is_upright(input.fall_detection) {
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

        let ground_origin = kinematics::sole_contact::ground_origin_in_leveled_robot(
            robot_kinematics.left_leg.sole_to_robot,
            robot_kinematics.right_leg.sole_to_robot,
            input.imu_state.roll_pitch_yaw.x(),
            input.imu_state.roll_pitch_yaw.y(),
        );
        let (Some(previous_contact), Some(previous_ground_origin)) =
            (self.previous_support_contact, self.previous_ground_origin)
        else {
            self.anchor(
                support_side,
                current_contact,
                ground_origin,
                yaw,
                input.time,
            );
            return Some(self.pose);
        };

        if self.support_side != Some(support_side) {
            self.anchor(
                support_side,
                current_contact,
                ground_origin,
                yaw,
                input.time,
            );
            return Some(self.pose);
        }

        let yaw_delta = normalize_angle(yaw - self.last_yaw.unwrap_or(yaw));
        let rotation = nalgebra::Rotation2::new(yaw_delta);
        let mut translation = previous_contact - Vector2::wrap(rotation * current_contact.inner);
        translation.inner.x *= parameters.translation_scale.x();
        translation.inner.y *= parameters.translation_scale.y();

        if self.delta_exceeds_limits(input.time, translation, yaw_delta, parameters) {
            self.anchor(
                support_side,
                current_contact,
                ground_origin,
                yaw,
                input.time,
            );
            return Some(self.pose);
        }

        let current_to_previous = Isometry2::<Ground, Ground>::from_parts(
            Vector2::wrap(
                translation.inner + rotation * ground_origin.inner - previous_ground_origin.inner,
            ),
            yaw_delta,
        );
        let previous_to_odometry = self.pose.as_transform::<Ground>();
        let current_to_odometry = previous_to_odometry * current_to_previous;
        self.pose = pose_with_yaw(current_to_odometry.as_pose(), yaw);

        self.previous_support_contact = Some(current_contact);
        self.previous_ground_origin = Some(ground_origin);
        self.last_yaw = Some(yaw);
        self.last_time = Some(input.time);

        Some(self.pose)
    }

    fn clear_contact_tracking(&mut self) {
        self.support_side = None;
        self.previous_support_contact = None;
        self.previous_ground_origin = None;
        self.last_yaw = None;
    }

    fn anchor(
        &mut self,
        support_side: SoleSide,
        contact: Vector2<Robot>,
        ground_origin: Vector2<Robot>,
        yaw: f32,
        time: Time,
    ) {
        self.support_side = Some(support_side);
        self.previous_support_contact = Some(contact);
        self.previous_ground_origin = Some(ground_origin);
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

fn is_upright(fall_detection: Option<&FallDetection>) -> bool {
    matches!(
        fall_detection,
        Some(FallDetection {
            posture: Posture::Upright,
            ..
        })
    )
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use booster::ImuState;
    use coordinate_systems::{LeftSole, RightSole};
    use kinematics::robot_kinematics::{
        RobotKinematics, RobotLeftLegKinematics, RobotRightLegKinematics,
    };
    use linear_algebra::{IntoTransform, point, vector};
    use ros_z::time::Time;
    use types::fall_detection::{FallDetection, Posture};

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

    fn upright() -> FallDetection {
        FallDetection {
            time: Time::default(),
            valid_until: Time::default(),
            posture: Posture::Upright,
        }
    }

    fn fallen() -> FallDetection {
        FallDetection {
            time: Time::default(),
            valid_until: Time::default(),
            posture: Posture::Fallen {
                is_ready_for_standup: false,
            },
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
                    fall_detection: Some(&upright()),
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
                fall_detection: Some(&upright()),
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
                    fall_detection: Some(&upright()),
                },
                &parameters,
            )
            .expect("second frame publishes");

        assert_relative_eq!(pose.position().x(), -0.025, epsilon = 1e-5);
    }

    /// Feet standing in the odometry frame, seen from a robot at `body` facing along `yaw`.
    fn kinematics_of_feet(
        body: nalgebra::Vector2<f32>,
        yaw: f32,
        left: nalgebra::Vector2<f32>,
        right: nalgebra::Vector2<f32>,
        left_height: f32,
        right_height: f32,
    ) -> RobotKinematics {
        let to_robot = nalgebra::Rotation2::new(-yaw);
        let left = to_robot * (left - body);
        let right = to_robot * (right - body);
        kinematics_with_positions(left, left_height, right, right_height)
    }

    fn kinematics_with_positions(
        left: nalgebra::Vector2<f32>,
        left_z: f32,
        right: nalgebra::Vector2<f32>,
        right_z: f32,
    ) -> RobotKinematics {
        RobotKinematics {
            left_leg: RobotLeftLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(left.x, left.y, left_z)
                    .framed_transform(),
                ..Default::default()
            },
            right_leg: RobotRightLegKinematics {
                sole_to_robot: nalgebra::Isometry3::translation(right.x, right.y, right_z)
                    .framed_transform(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// One support phase: the left foot stands while the robot sways sideways and turns, and the
    /// right foot swings forward.
    #[test]
    fn published_pose_follows_the_point_between_the_feet() {
        let mut estimator = OdometryEstimator::default();
        let mut parameters = parameters();
        parameters.max_angular_speed = 100.0;
        let left = nalgebra::vector![0.0, 0.1];
        let right_start = nalgebra::vector![-0.1, -0.1];
        let state_at = |index: usize| {
            let progress = index as f32 / 50.0;
            let body = nalgebra::vector![0.1 * progress, 0.04 * progress];
            let yaw = 0.3 * progress;
            let right = right_start + nalgebra::vector![0.3 * progress, 0.0];
            (body, yaw, right)
        };
        let mut first_ground = None;
        for index in 0..=50 {
            let (body, yaw, right) = state_at(index);
            let feet = kinematics_of_feet(body, yaw, left, right, -0.5, -0.45);
            let imu = imu(yaw);
            let pose = estimator
                .update(
                    EstimatorInput {
                        time: Time::from_nanos(1_000_000_000 + index as i64 * 2_000_000),
                        imu_state: &imu,
                        robot_kinematics: Some(&feet),
                        fall_detection: Some(&upright()),
                    },
                    &parameters,
                )
                .expect("valid frame publishes");

            // The odometry frame of the estimator starts at the ground frame of the first sample.
            let ground = (left + right) / 2.0;
            let first_ground = *first_ground.get_or_insert(ground);
            let expected = ground - first_ground;
            assert_relative_eq!(pose.position().x(), expected.x, epsilon = 1.0e-5);
            assert_relative_eq!(pose.position().y(), expected.y, epsilon = 1.0e-5);
            assert_relative_eq!(pose.angle(), yaw, epsilon = 1.0e-5);
        }
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
                fall_detection: Some(&upright()),
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
                    fall_detection: Some(&upright()),
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
                fall_detection: Some(&upright()),
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
                    fall_detection: Some(&upright()),
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
                fall_detection: Some(&upright()),
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
                    fall_detection: Some(&fallen()),
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
                fall_detection: Some(&upright()),
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
                    fall_detection: None,
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
                fall_detection: Some(&upright()),
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
                fall_detection: Some(&upright()),
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
                    fall_detection: Some(&upright()),
                },
                &parameters(),
            )
            .expect("first valid frame publishes");

        assert!(pose.orientation().angle().abs() < 1.0e-6);
    }
}
