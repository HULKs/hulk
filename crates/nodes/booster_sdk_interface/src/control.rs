use std::{f32::consts::PI, time::SystemTime};

use booster::Kick;
use booster_sdk::types::RobotMode as SdkRobotMode;
use linear_algebra::{Orientation2, Point2};
use ros2::std_msgs::header::Header;
use types::{
    motion_command::{KickPower, MotionCommand, OrientationMode},
    parameters::{BoosterKickingParameters, RLWalkingParameters},
    path::traits::{Length, PathProgress},
    step::Step,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DesiredMode {
    Damping,
    Prepare,
    Walking,
}

pub fn desired_mode_for(command: &MotionCommand, emergency_damping: bool) -> DesiredMode {
    if emergency_damping {
        return DesiredMode::Damping;
    }

    match command {
        MotionCommand::Damping => DesiredMode::Damping,
        MotionCommand::Prepare | MotionCommand::StandUp => DesiredMode::Prepare,
        MotionCommand::Stand { .. }
        | MotionCommand::VisualKick { .. }
        | MotionCommand::Walk { .. }
        | MotionCommand::WalkWithVelocity { .. } => DesiredMode::Walking,
    }
}

pub fn confirmed_mode_allows_walking(mode: Option<SdkRobotMode>) -> bool {
    matches!(mode, Some(SdkRobotMode::Walking))
}

pub fn target_alignment_importance(
    distance_to_be_aligned: f32,
    hybrid_align_distance: f32,
    distance_to_target: f32,
) -> f32 {
    if distance_to_target < distance_to_be_aligned {
        1.0
    } else if distance_to_target < distance_to_be_aligned + hybrid_align_distance {
        (1.0 + f32::cos(PI * (distance_to_target - distance_to_be_aligned) / hybrid_align_distance))
            * 0.5
    } else {
        0.0
    }
}

pub fn step_from_motion_command(command: &MotionCommand, parameters: &RLWalkingParameters) -> Step {
    match command {
        MotionCommand::Walk {
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            speed,
            ..
        } => {
            let forward = path.forward(Point2::origin());
            let distance_to_target = path.length();
            let deceleration_factor =
                (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
            let velocity = forward * *speed * deceleration_factor;

            let walk_orientation = match orientation_mode {
                OrientationMode::Unspecified | OrientationMode::AlignWithPath => {
                    Orientation2::from_vector(forward)
                }
                OrientationMode::LookTowards { direction, .. } => *direction,
                OrientationMode::LookAt { target, .. } => {
                    Orientation2::from_vector(*target - Point2::origin())
                }
            };

            let target_alignment_importance = target_alignment_importance(
                *distance_to_be_aligned,
                parameters.hybrid_align_distance,
                distance_to_target,
            );

            let orientation =
                walk_orientation.slerp(*target_orientation, target_alignment_importance);
            let angular_velocity = orientation.as_unit_vector().y() * parameters.max_alignment_rate;

            Step {
                forward: velocity.x(),
                left: velocity.y(),
                turn: angular_velocity,
            }
        }
        MotionCommand::WalkWithVelocity {
            velocity,
            angular_velocity,
            ..
        } => Step {
            forward: velocity.x(),
            left: velocity.y(),
            turn: *angular_velocity,
        },
        MotionCommand::Stand { .. }
        | MotionCommand::Damping
        | MotionCommand::Prepare
        | MotionCommand::StandUp
        | MotionCommand::VisualKick { .. } => Step::ZERO,
    }
}

pub fn kick_from_motion_command(
    command: &MotionCommand,
    stamp: SystemTime,
    parameters: &BoosterKickingParameters,
) -> Option<Kick> {
    let MotionCommand::VisualKick {
        ball_position,
        kick_direction,
        target_position,
        robot_theta_to_field,
        kick_power,
        ..
    } = command
    else {
        return None;
    };

    let kick_power = match kick_power {
        KickPower::Rumpelstilzchen => parameters.kick_power.rumpelstilzchen,
        KickPower::Schlong => parameters.kick_power.schlong,
    };

    Some(Kick {
        header: Header {
            stamp: stamp.into(),
            frame_id: String::new(),
        },
        ball_position_x: ball_position.x() as f64,
        ball_position_y: ball_position.y() as f64,
        kick_direction_angle: kick_direction.angle() as f64,
        target_position_x: target_position.x() as f64,
        target_position_y: target_position.y() as f64,
        robot_angle_to_field: robot_theta_to_field.angle() as f64,
        kick_power,
    })
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct VisualKickState {
    active: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VisualKickTransition {
    None,
    Start,
    Stop,
}

impl VisualKickState {
    pub fn is_active(self) -> bool {
        self.active
    }

    pub fn update(&mut self, should_be_active: bool) -> VisualKickTransition {
        match (self.active, should_be_active) {
            (false, true) => {
                self.active = true;
                VisualKickTransition::Start
            }
            (true, false) => {
                self.active = false;
                VisualKickTransition::Stop
            }
            _ => VisualKickTransition::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{point, vector};
    use types::{motion_command::HeadMotion, path::direct_path};

    fn walking_parameters() -> RLWalkingParameters {
        RLWalkingParameters {
            hybrid_align_distance: 1.0,
            max_alignment_rate: 2.0,
            deceleration_distance: 0.5,
            ..Default::default()
        }
    }

    #[test]
    fn visual_kick_command_builds_booster_kick_message() {
        use std::time::UNIX_EPOCH;
        use types::motion_command::KickPower;
        use types::parameters::{BoosterKickingParameters, KickPowerParameters};

        let command = MotionCommand::VisualKick {
            head: HeadMotion::ZeroAngles,
            ball_position: point![1.0, -0.5],
            kick_direction: Orientation2::new(0.25),
            target_position: point![4.0, 0.5],
            robot_theta_to_field: Orientation2::new(-0.2),
            kick_power: KickPower::Schlong,
        };
        let parameters = BoosterKickingParameters {
            kick_message_interval: Default::default(),
            kick_power: KickPowerParameters {
                rumpelstilzchen: 1.5,
                schlong: 6.0,
            },
        };

        let kick = kick_from_motion_command(&command, UNIX_EPOCH, &parameters).unwrap();

        assert_eq!(kick.ball_position_x, 1.0);
        assert_eq!(kick.ball_position_y, -0.5);
        assert_eq!(kick.target_position_x, 4.0);
        assert_eq!(kick.target_position_y, 0.5);
        assert_eq!(kick.kick_power, 6.0);
    }

    #[test]
    fn emergency_damping_overrides_walk_command() {
        let command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![1.0, 0.0],
            angular_velocity: 0.0,
        };

        assert_eq!(desired_mode_for(&command, true), DesiredMode::Damping);
    }

    #[test]
    fn walking_commands_request_walking_mode() {
        let command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![1.0, 0.0],
            angular_velocity: 0.0,
        };

        assert_eq!(desired_mode_for(&command, false), DesiredMode::Walking);
    }

    #[test]
    fn prepare_requests_prepare_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::Prepare, false),
            DesiredMode::Prepare
        );
    }

    #[test]
    fn stand_up_requests_prepare_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::StandUp, false),
            DesiredMode::Prepare
        );
    }

    #[test]
    fn damping_requests_damping_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::Damping, false),
            DesiredMode::Damping
        );
    }

    #[test]
    fn only_confirmed_walking_allows_walking_effects() {
        assert!(confirmed_mode_allows_walking(Some(SdkRobotMode::Walking)));
        assert!(!confirmed_mode_allows_walking(Some(SdkRobotMode::Prepare)));
        assert!(!confirmed_mode_allows_walking(None));
    }

    #[test]
    fn walk_with_velocity_maps_directly_to_step() {
        let command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![0.3, -0.2],
            angular_velocity: 0.4,
        };
        let step = step_from_motion_command(&command, &walking_parameters());

        assert_eq!(step.forward, 0.3);
        assert_eq!(step.left, -0.2);
        assert_eq!(step.turn, 0.4);
    }

    #[test]
    fn stand_maps_to_zero_step() {
        let command = MotionCommand::Stand {
            head: HeadMotion::ZeroAngles,
        };
        let step = step_from_motion_command(&command, &walking_parameters());

        assert_eq!(step.forward, 0.0);
        assert_eq!(step.left, 0.0);
        assert_eq!(step.turn, 0.0);
    }

    #[test]
    fn target_alignment_importance_is_one_inside_align_distance() {
        assert_eq!(target_alignment_importance(1.0, 2.0, 0.5), 1.0);
    }

    #[test]
    fn target_alignment_importance_is_zero_outside_hybrid_range() {
        assert_eq!(target_alignment_importance(1.0, 2.0, 4.0), 0.0);
    }

    #[test]
    fn target_alignment_importance_blends_with_cosine() {
        let importance = target_alignment_importance(1.0, 2.0, 2.0);
        assert!((importance - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn walk_path_decelerates_near_target() {
        let command = MotionCommand::Walk {
            head: HeadMotion::ZeroAngles,
            path: direct_path(point![0.0, 0.0], point![0.25, 0.0]),
            orientation_mode: OrientationMode::AlignWithPath,
            target_orientation: Orientation2::new(0.0),
            distance_to_be_aligned: 0.0,
            speed: 1.0,
        };

        let step = step_from_motion_command(&command, &walking_parameters());

        assert!((step.forward - 0.5).abs() < 0.001);
    }

    #[test]
    fn visual_kick_edges_track_activation() {
        let mut state = VisualKickState::default();

        assert_eq!(state.update(false), VisualKickTransition::None);
        assert_eq!(state.update(true), VisualKickTransition::Start);
        assert!(state.is_active());
        assert_eq!(state.update(true), VisualKickTransition::None);
        assert_eq!(state.update(false), VisualKickTransition::Stop);
        assert!(!state.is_active());
        assert_eq!(state.update(false), VisualKickTransition::None);
    }
}
