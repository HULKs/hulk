use std::time::Duration;

use ros_z::time::Time;
use types::{
    fall_detection::{FallDetection, Posture},
    motion_command::MotionCommand,
    robot_command::RobotCommand,
};

pub(super) fn command_allowed(
    command: &MotionCommand,
    fall: Option<&FallDetection>,
    now: Time,
) -> bool {
    if matches!(command, MotionCommand::Damping | MotionCommand::Prepare) {
        return true;
    }
    let Some(fall) = fall.filter(|fall| fall.is_fresh(now)) else {
        return false;
    };
    match command {
        MotionCommand::StandUp { .. } => matches!(
            fall.posture,
            Posture::Fallen {
                ready_for_standup: true
            } | Posture::StandingUp
        ),
        _ => fall.posture == Posture::Upright,
    }
}

pub(super) fn validate_dispatch(
    output: RobotCommand,
    inferred_request: &MotionCommand,
    latest_request: Option<(Time, &MotionCommand)>,
    fall: Option<&FallDetection>,
    now: Time,
    maximum_command_age: Duration,
) -> RobotCommand {
    let Some((_, request)) = latest_request
        .filter(|(time, _)| *time <= now && now.duration_since(*time) <= maximum_command_age)
    else {
        return RobotCommand::Damping;
    };
    match request {
        MotionCommand::Damping => RobotCommand::Damping,
        MotionCommand::Prepare => RobotCommand::Prepare,
        _ if command_allowed(inferred_request, fall, now)
            && command_allowed(request, fall, now) =>
        {
            output
        }
        _ => RobotCommand::Damping,
    }
}
