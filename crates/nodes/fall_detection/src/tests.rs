use kinematics::joints::leg::LegJoints;
use types::fall_detection::Posture::{Falling, StandingUp, Upright};

use super::*;
use crate::parameters::FallingParameters;

const SAMPLE_INTERVAL_MILLISECONDS: i64 = 10;

fn parameters() -> Parameters {
    Parameters {
        maximum_observation_age: Duration::from_millis(50),
        detection_lifetime: Duration::from_millis(100),
        upright: UprightParameters {
            maximum_torso_tilt: 0.35,
            maximum_torso_angular_velocity: 1.0,
            maximum_leg_joint_velocity: 2.0,
            minimum_torso_height_above_soles: 0.43,
            confirmation_duration: Duration::from_millis(100),
        },
        falling: FallingParameters {
            minimum_torso_tilt: 0.8,
            confirmation_duration: Duration::from_millis(20),
        },
        fallen: FallenParameters {
            minimum_torso_tilt: 1.0,
            maximum_torso_angular_velocity: 2.5,
            maximum_joint_velocity: 1.0,
            confirmation_duration: Duration::from_millis(150),
        },
        standing_up: StandingUpParameters {
            start_pose: start_pose(),
            maximum_start_pose_torso_deviation: 0.52,
            maximum_start_pose_torso_angular_velocity: 0.35,
            maximum_start_pose_joint_position_deviation: 0.3,
            maximum_start_pose_joint_velocity: 1.0,
            confirmation_duration: Duration::from_millis(150),
            movement_confirmation_duration: Duration::from_millis(50),
            timeout: Duration::from_secs(4),
        },
    }
}

fn legs(hip_pitch: f32, knee: f32, ankle_up: f32) -> BodyJoints<f32> {
    let leg = LegJoints {
        hip_pitch,
        hip_roll: 0.0,
        hip_yaw: 0.0,
        knee,
        ankle_up,
        ankle_down: 0.0,
    };
    BodyJoints {
        left_leg: leg,
        right_leg: leg,
        ..BodyJoints::fill(0.0)
    }
}

/// Slightly crouched, with the torso about 0.54 m above the soles.
fn start_pose() -> BodyJoints<f32> {
    legs(-0.2, 0.4, -0.25)
}

/// Far from the start pose, with folded legs.
fn twisted_pose() -> BodyJoints<f32> {
    start_pose() + BodyJoints::fill(1.0)
}

/// Hips and knees folded, with the torso about 0.36 m above the soles.
fn seated_pose() -> BodyJoints<f32> {
    legs(-1.6, 1.6, 0.0)
}

fn fallen(ready_for_standup: bool) -> Posture {
    Posture::Fallen { ready_for_standup }
}

#[derive(Clone, Copy)]
struct Sample {
    tilt: f32,
    angular_velocity: f32,
    positions: BodyJoints<f32>,
    velocities: BodyJoints<f32>,
}

impl Sample {
    fn standing() -> Self {
        Self {
            tilt: 0.0,
            angular_velocity: 0.0,
            positions: start_pose(),
            velocities: BodyJoints::fill(0.0),
        }
    }

    /// Between the upright and falling tilts, with the legs far from a standing pose.
    fn kicking() -> Self {
        Self {
            tilt: 0.5,
            positions: twisted_pose(),
            ..Self::standing()
        }
    }

    /// Between the falling and fallen tilts.
    fn tilted() -> Self {
        Self {
            tilt: 0.9,
            ..Self::standing()
        }
    }

    fn seated() -> Self {
        Self {
            positions: seated_pose(),
            ..Self::standing()
        }
    }

    fn lying_twisted() -> Self {
        Self {
            tilt: 1.5,
            positions: twisted_pose(),
            ..Self::standing()
        }
    }

    fn lying_in_start_pose() -> Self {
        Self {
            tilt: 1.5,
            ..Self::standing()
        }
    }

    fn rolling() -> Self {
        Self {
            angular_velocity: 3.0,
            ..Self::lying_twisted()
        }
    }

    /// The stand up policy has moved the joints away from the start pose.
    fn getting_up() -> Self {
        Self {
            positions: twisted_pose(),
            ..Self::tilted()
        }
    }
}

#[derive(Default)]
struct Robot {
    detector: Detector,
    milliseconds: i64,
    stand_up_requested: bool,
}

impl Robot {
    /// Feeds `sample` for `milliseconds` and returns the published postures without repeats.
    fn hold(&mut self, sample: Sample, milliseconds: i64) -> Vec<Posture> {
        let mut postures = Vec::new();
        for _ in 0..milliseconds / SAMPLE_INTERVAL_MILLISECONDS {
            self.milliseconds += SAMPLE_INTERVAL_MILLISECONDS;
            let observation = Observation {
                time: Time::from_nanos(self.milliseconds * 1_000_000),
                tilt: sample.tilt,
                pitch: sample.tilt,
                angular_velocity: sample.angular_velocity,
                positions: sample.positions,
                velocities: sample.velocities,
            };
            let Some(fall_detection) =
                self.detector
                    .update(observation, self.stand_up_requested, &parameters())
            else {
                continue;
            };
            if postures.last() != Some(&fall_detection.posture) {
                postures.push(fall_detection.posture);
            }
        }
        postures
    }

    /// Advances time without sensor samples.
    fn pause(&mut self, milliseconds: i64) {
        self.milliseconds += milliseconds;
    }

    fn standing_up() -> Self {
        let mut robot = Self::default();
        robot.hold(Sample::lying_in_start_pose(), 400);
        robot.stand_up_requested = true;
        assert_eq!(robot.hold(Sample::getting_up(), 10), [StandingUp]);
        robot
    }
}

#[test]
fn base_parameters_are_valid() {
    let parameters: Parameters = json5::from_str(include_str!(
        "../../../../etc/parameters/base/fall_detection.json5"
    ))
    .unwrap();
    assert_eq!(parameters.validate(), Ok(()));
}

#[test]
fn validation_rejects_overlapping_tilts() {
    let mut parameters = parameters();
    assert_eq!(parameters.validate(), Ok(()));
    parameters.falling.minimum_torso_tilt = parameters.fallen.minimum_torso_tilt;
    assert!(parameters.validate().is_err());
    parameters.falling.minimum_torso_tilt = f32::NAN;
    assert!(parameters.validate().is_err());
}

#[test]
fn fall_prepare_and_stand_up() {
    let mut robot = Robot::default();
    assert_eq!(robot.hold(Sample::standing(), 200), [Upright]);
    assert_eq!(robot.hold(Sample::tilted(), 50), [Upright, Falling]);
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 300),
        [Falling, fallen(false)]
    );
    // Prepare reached the start pose and settled.
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 200),
        [fallen(false), fallen(true)]
    );
    robot.stand_up_requested = true;
    // Readiness survives the policy moving the joints before the request arrives.
    assert_eq!(robot.hold(Sample::getting_up(), 1000), [StandingUp]);
    // Rolling during recovery must not count as lying still.
    assert_eq!(robot.hold(Sample::rolling(), 1000), [StandingUp]);
    assert_eq!(robot.hold(Sample::standing(), 200), [StandingUp, Upright]);
}

#[test]
fn stand_up_requires_ready() {
    let mut robot = Robot::default();
    assert_eq!(robot.hold(Sample::lying_twisted(), 300), [fallen(false)]);
    robot.stand_up_requested = true;
    assert_eq!(robot.hold(Sample::lying_twisted(), 1000), [fallen(false)]);
}

#[test]
fn upright_only_ends_on_sustained_tilt() {
    let mut robot = Robot::default();
    robot.hold(Sample::standing(), 200);
    assert_eq!(robot.hold(Sample::kicking(), 1000), [Upright]);
    // Shorter than falling.confirmation_duration.
    assert_eq!(robot.hold(Sample::tilted(), 10), [Upright]);
    robot.hold(Sample::standing(), 10);
    assert_eq!(robot.hold(Sample::tilted(), 30), [Upright, Falling]);
}

#[test]
fn seated_robot_is_not_upright() {
    let mut robot = Robot::default();
    assert!(robot.hold(Sample::seated(), 500).is_empty());
    robot.hold(Sample::standing(), 200);
    robot.hold(Sample::tilted(), 50);
    assert_eq!(robot.hold(Sample::seated(), 500), [Falling]);
}

#[test]
fn rolling_robot_is_not_fallen() {
    let mut robot = Robot::default();
    robot.hold(Sample::standing(), 200);
    robot.hold(Sample::tilted(), 50);
    assert_eq!(robot.hold(Sample::rolling(), 500), [Falling]);
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 200),
        [Falling, fallen(false)]
    );
}

#[test]
fn stand_up_timeout_while_lying_still_ends_fallen() {
    let mut robot = Robot::standing_up();
    assert_eq!(robot.hold(Sample::lying_twisted(), 3990), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 10), [Falling]);
    // Still requested, but not in the start pose: no retry without Prepare.
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 200),
        [Falling, fallen(false)]
    );
}

#[test]
fn stand_up_timeout_while_seated_damps() {
    let mut robot = Robot::standing_up();
    assert_eq!(robot.hold(Sample::seated(), 3990), [StandingUp]);
    assert_eq!(robot.hold(Sample::seated(), 10), [Falling]);
}

#[test]
fn stand_up_waits_in_start_pose_until_timeout_without_confirmed_movement() {
    let mut robot = Robot::standing_up();
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 3990),
        [StandingUp]
    );
    assert_eq!(robot.hold(Sample::lying_in_start_pose(), 10), [Falling]);
}

#[test]
fn stand_up_returns_fallen_after_each_kind_of_confirmed_movement() {
    let mut leg_motion = Sample::lying_twisted();
    leg_motion.velocities.left_leg.knee = -parameters().fallen.maximum_joint_velocity;
    let mut arm_motion = Sample::lying_twisted();
    arm_motion.velocities.right_arm.elbow = parameters().fallen.maximum_joint_velocity;
    let torso_motion = Sample {
        angular_velocity: parameters().fallen.maximum_torso_angular_velocity,
        ..Sample::lying_twisted()
    };

    for movement in [Sample::getting_up(), torso_motion, leg_motion, arm_motion] {
        let mut robot = Robot::standing_up();
        // Six samples span the 50 ms needed to confirm movement.
        assert_eq!(robot.hold(movement, 60), [StandingUp]);
        assert_eq!(robot.hold(Sample::lying_twisted(), 150), [StandingUp]);
        assert_eq!(robot.hold(Sample::lying_twisted(), 10), [fallen(false)]);
        // A continued request cannot bypass preparation and fresh readiness.
        assert_eq!(robot.hold(Sample::lying_twisted(), 200), [fallen(false)]);
    }
}

#[test]
fn brief_stand_up_movement_does_not_enable_a_return_to_fallen() {
    let mut robot = Robot::standing_up();
    for duration in [10, 50, 50] {
        assert_eq!(robot.hold(Sample::rolling(), duration), [StandingUp]);
        assert_eq!(robot.hold(Sample::lying_twisted(), 200), [StandingUp]);
    }
    assert_eq!(robot.hold(Sample::rolling(), 60), [StandingUp]);
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 160),
        [StandingUp, fallen(false)]
    );
}

#[test]
fn stand_up_requires_continuous_settling_after_movement() {
    let mut robot = Robot::standing_up();
    assert_eq!(robot.hold(Sample::getting_up(), 60), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 150), [StandingUp]);
    assert_eq!(robot.hold(Sample::rolling(), 10), [StandingUp]);
    // Movement stays confirmed, but settling starts over.
    assert_eq!(robot.hold(Sample::lying_twisted(), 150), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 10), [fallen(false)]);
}

#[test]
fn short_sensor_gaps_restart_stand_up_confirmations() {
    let mut robot = Robot::standing_up();
    robot.hold(Sample::rolling(), 50);
    robot.pause(70);
    assert_eq!(robot.hold(Sample::rolling(), 50), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 200), [StandingUp]);

    robot.hold(Sample::rolling(), 60);
    robot.hold(Sample::lying_twisted(), 150);
    robot.pause(70);
    // Confirmed movement survives the gap; unfinished settling does not.
    assert_eq!(robot.hold(Sample::lying_twisted(), 150), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 10), [fallen(false)]);
}

#[test]
fn stand_up_retry_requires_new_movement() {
    let mut robot = Robot::standing_up();
    robot.hold(Sample::getting_up(), 60);
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 160),
        [StandingUp, fallen(false)]
    );

    robot.stand_up_requested = false;
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 160),
        [fallen(false), fallen(true)]
    );
    robot.stand_up_requested = true;
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 1000),
        [StandingUp]
    );
    robot.hold(Sample::rolling(), 60);
    assert_eq!(
        robot.hold(Sample::lying_twisted(), 160),
        [StandingUp, fallen(false)]
    );
}

#[test]
fn long_sensor_gap_discards_confirmed_stand_up_movement() {
    let mut robot = Robot::standing_up();
    robot.hold(Sample::getting_up(), 60);
    robot.pause(190);
    robot.stand_up_requested = false;
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 400),
        [fallen(false), fallen(true)]
    );
    robot.stand_up_requested = true;
    assert_eq!(
        robot.hold(Sample::lying_in_start_pose(), 1000),
        [StandingUp]
    );
}

#[test]
fn stand_up_timeout_does_not_wait_for_fallen_confirmation() {
    let mut robot = Robot::standing_up();
    assert_eq!(robot.hold(Sample::getting_up(), 3900), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 90), [StandingUp]);
    assert_eq!(robot.hold(Sample::lying_twisted(), 10), [Falling]);
}

#[test]
fn short_sensor_gap_keeps_posture() {
    let mut robot = Robot::default();
    robot.hold(Sample::standing(), 200);
    // An 80 ms gap: longer than maximum_observation_age, shorter than detection_lifetime.
    robot.pause(70);
    // Upright could not be confirmed again while kicking.
    assert_eq!(robot.hold(Sample::kicking(), 10), [Upright]);
}

#[test]
fn long_sensor_gap_resets_posture() {
    let mut robot = Robot::default();
    robot.hold(Sample::standing(), 200);
    // A 200 ms gap.
    robot.pause(190);
    assert!(robot.hold(Sample::standing(), 10).is_empty());
}
