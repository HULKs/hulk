use std::f32::consts::FRAC_PI_4;

use coordinate_systems::{Field, Ground};
use framework::AdditionalOutput;
use geometry::look_at::LookAt;
use hsl_network_messages::SubState;
use linear_algebra::{point, Pose2, Rotation2, Vector2};
use types::{
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

#[allow(clippy::too_many_arguments)]
pub fn execute(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    field_side: Option<Side>,
    distance_to_ball: f32,
    maximum_x_in_ready_and_when_ball_is_not_free: f32,
    minimum_x: f32,
    walk_and_stand: &WalkAndStand,
    look_action: &LookAction,
    path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    walk_speed: WalkSpeed,
    distance_to_be_aligned: f32,
) -> Option<MotionCommand> {
    let pose = support_pose(
        world_state,
        field_dimensions,
        field_side,
        distance_to_ball,
        maximum_x_in_ready_and_when_ball_is_not_free,
        minimum_x,
    )?;
    walk_and_stand.execute(
        pose,
        look_action.execute(),
        path_obstacles_output,
        walk_speed,
        OrientationMode::AlignWithPath,
        distance_to_be_aligned,
        walk_and_stand.parameters.supporter_hysteresis,
    )
}

fn support_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    field_side: Option<Side>,
    distance_to_ball: f32,
    maximum_x_in_ready_and_when_ball_is_not_free: f32,
    minimum_x: f32,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let ball = world_state
        .rule_ball
        .or(world_state.ball)
        .unwrap_or_else(|| BallState::new_at_center(ground_to_field));
    let distance_from_midline_to_penaltybox =
        field_dimensions.length / 2.0 - field_dimensions.penalty_area_length;
    let distance_from_midline_to_goalbox =
        field_dimensions.length / 2.0 - field_dimensions.goal_box_area_length;
    let maximum_x = if field_side.is_none() {
        distance_from_midline_to_penaltybox
    } else {
        distance_from_midline_to_goalbox
    };
    let side = field_side.unwrap_or_else(|| match world_state.filtered_game_controller_state {
        Some(FilteredGameControllerState {
            sub_state: Some(SubState::CornerKick),
            ..
        }) => ball.field_side,
        _ => ball.field_side.opposite(),
    });
    let offset_vector = Rotation2::new(match side {
        Side::Left => FRAC_PI_4,
        Side::Right => -FRAC_PI_4,
    }) * (Vector2::<Field>::x_axis() * distance_to_ball);
    let supporting_position = ball.ball_in_field + offset_vector;

    let filtered_game_state = world_state
        .filtered_game_controller_state
        .as_ref()
        .map(|filtered_game_controller_state| filtered_game_controller_state.game_state);
    let sub_state = world_state
        .filtered_game_controller_state
        .as_ref()
        .map(|filtered_game_controller_state| filtered_game_controller_state.sub_state);
    let mut clamped_x = match (filtered_game_state, sub_state) {
        (Some(FilteredGameState::Ready), Some(Some(SubState::PenaltyKick))) => {
            supporting_position.x().max(field_dimensions.length / 4.0)
        }
        (Some(FilteredGameState::Playing { .. }), Some(Some(SubState::GoalKick))) => {
            supporting_position.x().min(field_dimensions.length / 4.0)
        }
        (Some(FilteredGameState::Ready), _)
        | (
            Some(FilteredGameState::Playing {
                ball_is_free: false,
                kick_off: true,
                ..
            }),
            _,
        ) => supporting_position
            .x()
            .min(maximum_x_in_ready_and_when_ball_is_not_free),
        _ => supporting_position.x().clamp(minimum_x, maximum_x),
    };

    let clamped_y = if (distance_from_midline_to_penaltybox..=distance_from_midline_to_goalbox)
        .contains(&clamped_x)
    {
        let absolute_clamped_y_distance_a = field_dimensions.penalty_area_width / 2.0
            - (clamped_x - distance_from_midline_to_penaltybox);
        let absolute_clamped_y_distance_b =
            absolute_clamped_y_distance_a - field_dimensions.goal_box_area_width / 2.0;
        match side {
            Side::Left => supporting_position.y().clamp(
                -absolute_clamped_y_distance_b,
                absolute_clamped_y_distance_a,
            ),
            Side::Right => supporting_position.y().clamp(
                -absolute_clamped_y_distance_a,
                absolute_clamped_y_distance_b,
            ),
        }
    } else {
        supporting_position.y().clamp(
            -field_dimensions.penalty_area_width / 2.0,
            field_dimensions.penalty_area_width / 2.0,
        )
    };
    clamped_x = if let Some(FilteredGameControllerState {
        sub_state: Some(SubState::PenaltyKick),
        ..
    }) = world_state.filtered_game_controller_state
    {
        clamped_x.min(distance_from_midline_to_penaltybox - 0.5)
    } else {
        clamped_x
    };

    let clamped_position = point![clamped_x, clamped_y];
    let support_pose = Pose2::new(
        clamped_position,
        clamped_position.look_at(&ball.ball_in_field).angle(),
    );
    Some(ground_to_field.inverse() * support_pose)
}
