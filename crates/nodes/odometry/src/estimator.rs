use booster::{FallDownState, FallDownStateType, ImuState};
use coordinate_systems::{Ground, LeftSole, Odometry, RightSole, Robot};
use kinematics::{
    robot_kinematics::RobotKinematics,
    sole_contact::{SoleSide, SupportSelectionParameters},
};
use linear_algebra::{Isometry2, Point3, Pose2, Vector2};
use ros_z::{Message, time::Time};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use types::odometry::KinematicOdometryDelta;

const DELTA_INTERVAL: Duration = Duration::from_millis(50);
pub(crate) const MAX_SAMPLE_GAP: Duration = Duration::from_millis(100);

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
    /// Maximum disagreement between feet's displacement during double support, in metres.
    #[serde(default = "default_double_support_tolerance")]
    pub double_support_translation_tolerance: f32,
    /// Candidate left-sole vertices used to estimate the current contact point.
    pub left_sole_contact_vertices: Vec<Point3<LeftSole>>,
    /// Candidate right-sole vertices used to estimate the current contact point.
    pub right_sole_contact_vertices: Vec<Point3<RightSole>>,
}

fn default_double_support_tolerance() -> f32 {
    0.01
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
    previous_contacts: Option<[Vector2<Robot>; 2]>,
    previous_double_support: bool,
    pending_delta: Option<KinematicOdometryDelta>,
    delta: Option<KinematicOdometryDelta>,
}

impl OdometryEstimator {
    pub fn update(
        &mut self,
        input: EstimatorInput<'_>,
        parameters: &Parameters,
    ) -> Option<Pose2<Odometry>> {
        self.delta = None;
        let Some(robot_kinematics) = input.robot_kinematics else {
            self.clear_contact_tracking();
            return None;
        };
        if !input
            .imu_state
            .roll_pitch_yaw
            .inner
            .iter()
            .all(|v| v.is_finite())
            || [
                robot_kinematics.left_leg.sole_to_robot.inner,
                robot_kinematics.right_leg.sole_to_robot.inner,
            ]
            .iter()
            .any(|pose| !pose.to_homogeneous().iter().all(|v| v.is_finite()))
            || self.last_time.is_some_and(|last| input.time <= last)
        {
            self.clear_contact_tracking();
            return None;
        }
        if self
            .last_time
            .is_some_and(|last| input.time.duration_since(last) > MAX_SAMPLE_GAP)
        {
            self.clear_contact_tracking();
        }

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
        );
        let right_contact = kinematics::sole_contact::estimate_sole_contact(
            robot_kinematics.right_leg.sole_to_robot,
            input.imu_state.roll_pitch_yaw.x(),
            input.imu_state.roll_pitch_yaw.y(),
            &parameters.right_sole_contact_vertices,
            parameters.contact_height_epsilon,
        );
        let (Some(left_contact), Some(right_contact)) = (left_contact, right_contact) else {
            self.clear_contact_tracking();
            return None;
        };
        let double_support =
            (left_contact.min_z - right_contact.min_z).abs() < parameters.double_support_deadband;
        let support_side = kinematics::sole_contact::select_support_side(
            left_contact,
            right_contact,
            self.support_side,
            SupportSelectionParameters {
                double_support_deadband: parameters.double_support_deadband,
                support_switch_hysteresis: parameters.support_switch_hysteresis,
            },
        );

        let support_side = support_side.unwrap_or(self.support_side.unwrap_or(SoleSide::Left));
        let contacts = [
            left_contact.contact_xy_in_leveled_robot,
            right_contact.contact_xy_in_leveled_robot,
        ];
        let side = usize::from(support_side == SoleSide::Right);
        let Some(previous_contacts) = self.previous_contacts else {
            self.anchor(support_side, contacts, yaw, input.time, double_support);
            return Some(self.pose);
        };

        if self.support_side != Some(support_side) {
            self.anchor(support_side, contacts, yaw, input.time, double_support);
            return Some(self.pose);
        }

        let yaw_delta = normalize_angle(yaw - self.last_yaw.unwrap_or(yaw));
        let rotation = nalgebra::Rotation2::new(yaw_delta);
        let translations = std::array::from_fn::<_, 2, _>(|i| {
            previous_contacts[i] - Vector2::wrap(rotation * contacts[i].inner)
        });
        if double_support
            && self.previous_double_support
            && (translations[0] - translations[1]).norm()
                > parameters.double_support_translation_tolerance
        {
            self.anchor(support_side, contacts, yaw, input.time, double_support);
            return Some(self.pose);
        }
        let mut translation = translations[side];
        translation.inner.x *= parameters.translation_scale.x();
        translation.inner.y *= parameters.translation_scale.y();

        if !translation.inner.iter().all(|v| v.is_finite())
            || self.delta_exceeds_limits(input.time, translation, yaw_delta, parameters)
        {
            self.anchor(support_side, contacts, yaw, input.time, double_support);
            return Some(self.pose);
        }

        let current_to_previous =
            Isometry2::<Ground, Ground>::from_parts(Vector2::wrap(translation.inner), yaw_delta);
        let previous_to_odometry = self.pose.as_transform::<Ground>();
        let current_to_odometry = previous_to_odometry * current_to_previous;
        self.pose = pose_with_yaw(current_to_odometry.as_pose(), yaw);

        let pending = self.pending_delta.get_or_insert(KinematicOdometryDelta {
            previous_time: self.last_time?,
            time: input.time,
            current_to_previous: Isometry2::identity(),
        });
        pending.current_to_previous = pending.current_to_previous * current_to_previous;
        pending.time = input.time;
        if pending.time.duration_since(pending.previous_time) >= DELTA_INTERVAL {
            self.delta = self.pending_delta.take();
        }
        self.previous_contacts = Some(contacts);
        self.previous_double_support = double_support;
        self.last_yaw = Some(yaw);
        self.last_time = Some(input.time);

        Some(self.pose)
    }

    fn clear_contact_tracking(&mut self) {
        self.support_side = None;
        self.previous_contacts = None;
        self.last_yaw = None;
        self.pending_delta = None;
    }

    /// Take the completed contact interval from the latest update, if any.
    pub fn take_delta(&mut self) -> Option<KinematicOdometryDelta> {
        self.delta.take()
    }

    fn anchor(
        &mut self,
        support_side: SoleSide,
        contacts: [Vector2<Robot>; 2],
        yaw: f32,
        time: Time,
        double_support: bool,
    ) {
        self.pending_delta = None;
        self.support_side = Some(support_side);
        self.previous_contacts = Some(contacts);
        self.previous_double_support = double_support;
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
            double_support_translation_tolerance: 0.01,
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

    #[test]
    fn measured_stops_in_double_support_are_distinct_from_rejected_intervals() {
        let mut estimator = OdometryEstimator::default();
        let parameters = parameters();
        let imu = imu(0.0);
        let ready = ready();
        let fallen = fallen();
        let mut update =
            |milliseconds: i64, x, right_x, height, valid: bool, state: &FallDownState| {
                let feet = kinematics_with_heights(x, height, right_x, 0.0);
                estimator.update(
                    EstimatorInput {
                        time: Time::from_nanos(milliseconds * 1_000_000),
                        imu_state: &imu,
                        robot_kinematics: valid.then_some(&feet),
                        fall_down_state: Some(state),
                    },
                    &parameters,
                );
                estimator.take_delta()
            };
        assert!(update(1000, 0.0, 0.0, -0.02, true, &ready).is_none());
        let walking = update(1050, -0.02, 0.0, -0.02, true, &ready).unwrap();
        assert!((walking.current_to_previous.translation().x() - 0.02).abs() < 1e-6);
        let stopped = update(1100, -0.02, 0.0, 0.0, true, &ready).unwrap();
        assert!(stopped.current_to_previous.translation().coords().norm() < 1e-6);
        assert!(update(1150, -0.02, 0.0, 0.0, true, &ready).is_some());
        // Both feet must agree before a double-support displacement is accepted.
        assert!(update(1200, -0.1, 0.0, 0.0, true, &ready).is_none());
        // A support switch reanchors rather than measuring zero motion.
        assert!(update(1250, -0.1, 0.0, 0.02, true, &ready).is_none());
        assert!(update(1300, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1350, -0.1, -5.0, 0.02, false, &ready).is_none());
        assert!(update(1400, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1450, -0.1, -5.0, 0.02, true, &fallen).is_none());
        assert!(update(1500, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1500, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1550, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1800, -0.1, -5.0, 0.02, true, &ready).is_none());
        let resumed = update(1850, -0.1, -5.0, 0.02, true, &ready).unwrap();
        assert_eq!(resumed.previous_time, Time::from_nanos(1_800_000_000));
        assert!(update(1860, -0.1, -5.0, 0.02, true, &ready).is_none());
        assert!(update(1870, -0.1, -5.0, 0.02, false, &ready).is_none());
        assert!(update(1880, -0.1, -5.0, 0.02, true, &ready).is_none());
        let resumed = update(1930, -0.1, -5.0, 0.02, true, &ready).unwrap();
        assert_eq!(resumed.previous_time, Time::from_nanos(1_880_000_000));
    }
}
