use coordinate_systems::Ground;
use framework::AdditionalOutput;
use hsl_network_messages::{SubState, Team};
use linear_algebra::{Pose2, point};
use types::{
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::MotionCommand,
    parameters::RolePositionsParameters,
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::{core::Defend, left::block_on_circle};

impl<'cycle> Defend<'cycle> {
    pub fn penalty_kick(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: f32,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose =
            defend_penalty_kick(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
            self.walk_and_stand.parameters.defender_hysteresis,
        )
    }
}

fn defend_penalty_kick(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));

    let position_to_defend = point![
        (-field_dimensions.length + field_dimensions.penalty_area_length) / 2.0,
        0.0
    ];
    let mut distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };
    distance_to_target = penalty_kick_defender_radius(
        distance_to_target,
        world_state.filtered_game_controller_state.as_ref(),
        field_dimensions,
    );

    let defend_pose = block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);
    Some(ground_to_field.inverse() * defend_pose)
}

fn penalty_kick_defender_radius(
    distance_to_target: f32,
    filtered_game_controller_state: Option<&FilteredGameControllerState>,
    field_dimensions: &FieldDimensions,
) -> f32 {
    if let Some(FilteredGameControllerState {
        kicking_team: Some(Team::Opponent),
        sub_state: Some(SubState::PenaltyKick),
        ..
    }) = filtered_game_controller_state
    {
        let half_penalty_width = field_dimensions.penalty_area_width / 2.0;
        let minimum_penalty_defender_radius =
            nalgebra::vector![field_dimensions.penalty_area_length, half_penalty_width].norm();
        distance_to_target.max(minimum_penalty_defender_radius)
    } else {
        distance_to_target
    }
}
