
use std::ops::Range;

use coordinate_systems::{Field, Ground};
use framework::AdditionalOutput;
use geometry::{line::{Line, Line2}, look_at::LookAt};
use hsl_network_messages::{GamePhase, SubState, Team};
use linear_algebra::{Point2, Pose2, Vector2, point};
use types::{
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{MotionCommand, WalkSpeed},
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::defend::Defend;

impl<'cycle> Defend<'cycle> {
    pub fn goal(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose = defend_goal_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
            self.walk_and_stand.parameters.hysteresis,
        )
    }
}

fn defend_goal_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let keeper_x_offset = match world_state.filtered_game_controller_state {
        Some(
            FilteredGameControllerState {
                game_phase:
                    GamePhase::PenaltyShootout {
                        kicking_team: Team::Opponent,
                    },
                ..
            }
            | FilteredGameControllerState {
                sub_state: Some(SubState::PenaltyKick),
                kicking_team: Some(Team::Opponent),
                ..
            },
        ) => 0.0,
        _ => role_positions.keeper_x_offset,
    };

    let passive_position_to_defend = point![-field_dimensions.length / 2.0 + 0.25, 0.0];

    if ball.ball_in_ground.coords().norm() >= role_positions.keeper_passive_distance {
        return Some(
            ground_to_field.inverse() * Pose2::<Field>::new(passive_position_to_defend, 0.0),
        );
    }

    let position_to_defend = point![-field_dimensions.length / 2.0 - 1.0, 0.0];

    let defend_pose = block_on_line(
        ball.ball_in_field,
        position_to_defend,
        -field_dimensions.length / 2.0 + keeper_x_offset,
        -0.7..0.7,
    );
    Some(ground_to_field.inverse() * defend_pose)
}


fn block_on_line(
    ball_position: Point2<Field>,
    target: Point2<Field>,
    defense_line_x: f32,
    defense_line_y_range: Range<f32>,
) -> Pose2<Field> {
    let is_ball_in_front_of_defense_line = defense_line_x < ball_position.x();
    if is_ball_in_front_of_defense_line {
        let defense_line = Line {
            point: point![defense_line_x, 0.0],
            direction: Vector2::y_axis(),
        };
        let ball_target_line = Line2::from_points(ball_position, target);
        let intersection_point = defense_line.intersection(&ball_target_line);
        let defense_position = point![
            intersection_point.x(),
            intersection_point
                .y()
                .clamp(defense_line_y_range.start, defense_line_y_range.end)
        ];
        Pose2::new(
            defense_position,
            defense_position.look_at(&ball_position).angle(),
        )
    } else {
        let defense_position = point![
            defense_line_x,
            (defense_line_y_range.start + defense_line_y_range.end) / 2.0
        ];
        Pose2::new(
            defense_position,
            defense_position.look_at(&ball_position).angle(),
        )
    }
}
