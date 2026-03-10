use coordinate_systems::{Field, Ground};
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::AdditionalOutput;
use geometry::look_at::LookAt;
use hsl_network_messages::{SubState, Team};
use linear_algebra::{Point2, Pose2, Vector2, point};
use types::{
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{MotionCommand, WalkSpeed},
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::core::{Defend, DefendMode};

impl<'cycle> Defend<'cycle> {
    pub fn left(
        &mut self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose = defend_pose(
            self.world_state,
            self.field_dimensions,
            self.role_positions,
            -self.field_dimensions.length / 2.0,
            Side::Left,
            self.last_defender_mode,
        )?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
            self.walk_and_stand.parameters.defender_hysteresis,
        )
    }
}

pub fn defend_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
    x_offset: f32,
    field_side: Side,
    last_defender_mode: &mut DefendMode,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let y_offset = if field_side == Side::Right {
        -role_positions.defender_y_offset
    } else {
        role_positions.defender_y_offset
    };
    let position_to_defend = point![x_offset, y_offset];
    let mode = if greater_than_with_hysteresis(
        *last_defender_mode == DefendMode::Passive,
        ball.ball_in_ground.coords().norm(),
        role_positions.defender_passive_distance,
        role_positions.defender_passive_hysteresis,
    ) {
        DefendMode::Passive
    } else {
        DefendMode::Aggressive
    };
    *last_defender_mode = mode;

    if mode == DefendMode::Passive {
        let passive_target_position = position_to_defend
            + (Vector2::x_axis() * role_positions.defender_aggressive_ring_radius);
        return Some(
            ground_to_field.inverse()
                * Pose2::<Field>::new(
                    passive_target_position,
                    passive_target_position.look_at(&ball.ball_in_field).angle(),
                ),
        );
    }

    let distance_to_target = if field_side == ball.field_side {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let position_to_defend_to_ball_max_length =
        (ball.ball_in_field - position_to_defend).norm() * (2.0 / 3.0);
    let distance_to_target = distance_to_target.min(position_to_defend_to_ball_max_length);

    let mut defend_pose =
        block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);

    if let Some(FilteredGameControllerState {
        kicking_team: Some(Team::Opponent),
        sub_state: Some(SubState::PenaltyKick),
        ..
    }) = world_state.filtered_game_controller_state
    {
        let x_position = x_offset + field_dimensions.penalty_area_length + 0.5;
        let penalty_kick_position = point![x_position, y_offset];
        defend_pose = Pose2::new(
            penalty_kick_position,
            penalty_kick_position.look_at(&ball.ball_in_field).angle(),
        )
    }

    Some(ground_to_field.inverse() * defend_pose)
}

pub fn block_on_circle(
    ball_position: Point2<Field>,
    target: Point2<Field>,
    distance_to_target: f32,
) -> Pose2<Field> {
    let target_to_ball = ball_position - target;
    let block_position = target + (target_to_ball.normalize() * distance_to_target);
    Pose2::new(
        block_position,
        block_position.look_at(&ball_position).angle(),
    )
}
