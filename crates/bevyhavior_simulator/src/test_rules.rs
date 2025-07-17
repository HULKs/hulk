use bevy::ecs::system::{Query, ResMut};
use types::{motion_command::MotionCommand, planned_path::PathSegment, roles::Role};

use crate::{game_controller::GameController, robot::Robot, soft_error::SoftErrorSender};

pub fn check_robots_dont_walk_into_rule_obstacles(
    robots: Query<&Robot>,
    game_controller: ResMut<GameController>,
    mut soft_error: SoftErrorSender,
) {
    for robot in robots.iter() {
        let rule_obstacles = &robot.database.main_outputs.rule_obstacles;
        let motion_command = &robot.database.main_outputs.motion_command;
        let MotionCommand::Walk { path, .. } = motion_command else {
            continue;
        };
        let Some(PathSegment::LineSegment(segment)) = path.segments.last() else {
            continue;
        };
        let destination_in_field = robot.ground_to_field() * segment.1;

        if game_controller.state.sub_state == Some(spl_network_messages::SubState::PenaltyKick)
            && robot.database.main_outputs.role == Role::Striker
        {
            continue;
        }

        for obstacle in rule_obstacles {
            if obstacle.contains(destination_in_field) {
                soft_error.send(format!(
                    "Robot {} ran into rule obstacle",
                    robot.parameters.player_number
                ));
            }
        }
    }
}
