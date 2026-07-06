use booster::Kick;
use ros_z::time::Time;
use ros2::std_msgs::header::Header;
use types::{
    motion_command::{KickPower, MotionCommand},
    parameters::BoosterKickingParameters,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DesiredMode {
    Damping,
    Prepare,
    Soccer,
}

pub fn desired_mode_for(command: &MotionCommand) -> DesiredMode {
    match command {
        MotionCommand::Damping => DesiredMode::Damping,
        MotionCommand::Prepare => DesiredMode::Prepare,
        MotionCommand::Stand { .. }
        | MotionCommand::VisualKick { .. }
        | MotionCommand::Walk { .. }
        | MotionCommand::WalkWithVelocity { .. }
        | MotionCommand::StandUp => DesiredMode::Soccer,
    }
}

pub fn kick_from_motion_command(
    command: &MotionCommand,
    stamp: Time,
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
            stamp: stamp.to_wallclock().into(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::{Orientation2, point, vector};
    use types::motion_command::HeadMotion;

    #[test]
    fn visual_kick_command_builds_booster_kick_message() {
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

        let kick = kick_from_motion_command(&command, Time::zero(), &parameters).unwrap();

        assert_eq!(kick.ball_position_x, 1.0);
        assert_eq!(kick.ball_position_y, -0.5);
        assert_eq!(kick.target_position_x, 4.0);
        assert_eq!(kick.target_position_y, 0.5);
        assert_eq!(kick.kick_power, 6.0);
    }

    #[test]
    fn walking_commands_request_soccer_mode() {
        let command = MotionCommand::WalkWithVelocity {
            head: HeadMotion::ZeroAngles,
            velocity: vector![1.0, 0.0],
            angular_velocity: 0.0,
        };

        assert_eq!(desired_mode_for(&command), DesiredMode::Soccer);
    }

    #[test]
    fn prepare_requests_prepare_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::Prepare),
            DesiredMode::Prepare
        );
    }

    #[test]
    fn stand_up_requests_soccer_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::StandUp),
            DesiredMode::Soccer
        );
    }

    #[test]
    fn damping_requests_damping_mode() {
        assert_eq!(
            desired_mode_for(&MotionCommand::Damping),
            DesiredMode::Damping
        );
    }
}
