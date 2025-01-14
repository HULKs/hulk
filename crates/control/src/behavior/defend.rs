use std::ops::Range;

use coordinate_systems::{Field, Ground};
use framework::AdditionalOutput;
use geometry::{
    line::{Line, Line2},
    look_at::LookAt,
};
use linear_algebra::{distance, point, Point2, Pose2, Vector2};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{MotionCommand, WalkSpeed},
    parameters::{RolePositionsParameters, WideStanceParameters},
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub struct Defend<'cycle> {
    world_state: &'cycle WorldState,
    field_dimensions: &'cycle FieldDimensions,
    role_positions: &'cycle RolePositionsParameters,
    walk_and_stand: &'cycle WalkAndStand<'cycle>,
    look_action: &'cycle LookAction<'cycle>,
}

impl<'cycle> Defend<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        role_positions: &'cycle RolePositionsParameters,
        walk_and_stand: &'cycle WalkAndStand,
        look_action: &'cycle LookAction,
    ) -> Self {
        Self {
            world_state,
            field_dimensions,
            role_positions,
            walk_and_stand,
            look_action,
        }
    }

    fn with_pose(
        &self,
        pose: Pose2<Ground>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        self.walk_and_stand.execute(
            pose,
            self.look_action.execute(),
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }

    pub fn wide_stance(&self, parameters: WideStanceParameters) -> Option<MotionCommand> {
        let ball = self.world_state.ball?;

        let position = ball.ball_in_ground;
        let velocity = ball.ball_in_ground_velocity;

        let ball_is_in_front_of_robot =
            position.coords().norm() < parameters.maximum_ball_distance && position.x() > 0.0;
        let ball_is_moving_towards_robot =
            ball.ball_in_ground_velocity.x() < -parameters.minimum_ball_velocity;

        if !ball_is_in_front_of_robot || !ball_is_moving_towards_robot {
            return None;
        }

        let horizontal_distance_to_intersection =
            position.y() - position.x() / velocity.x() * velocity.y();

        if horizontal_distance_to_intersection.abs() < parameters.action_radius {
            Some(MotionCommand::WideStance)
        } else {
            None
        }
    }

    pub fn left(
        &self,
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
        )?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }

    pub fn right(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose = defend_pose(
            self.world_state,
            self.field_dimensions,
            self.role_positions,
            -self.field_dimensions.length / 2.0,
            Side::Right,
        )?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }

    pub fn opponent_corner_kick(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        field_side: Side,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose = defend_pose(
            self.world_state,
            self.field_dimensions,
            self.role_positions,
            -self.field_dimensions.length / 2.0 + self.field_dimensions.goal_box_area_length * 2.0,
            field_side,
        )?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }

    pub fn penalty_kick(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose =
            defend_penalty_kick(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }

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
        )
    }

    pub fn kick_off(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
    ) -> Option<MotionCommand> {
        let pose =
            defend_kick_off_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(
            pose,
            path_obstacles_output,
            walk_speed,
            distance_to_be_aligned,
        )
    }
}

fn defend_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
    x_offset: f32,
    field_side: Side,
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

    let in_passive_mode =
        ball.ball_in_ground.coords().norm() >= role_positions.defender_passive_distance;

    let mut distance_to_target = match (in_passive_mode, field_side, ball.field_side) {
        (true, _, _) => role_positions.defender_aggressive_ring_radius,
        (_, Side::Left, Side::Left) | (_, Side::Right, Side::Right) => {
            role_positions.defender_aggressive_ring_radius
        }
        _ => role_positions.defender_passive_ring_radius,
    };

    if in_passive_mode {
        let passive_target_position = position_to_defend + (Vector2::x_axis() * distance_to_target);
        return Some(
            ground_to_field.inverse()
                * Pose2::<Field>::new(
                    passive_target_position,
                    passive_target_position.look_at(&ball.ball_in_field).angle(),
                ),
        );
    }

    distance_to_target = penalty_kick_defender_radius(
        distance_to_target,
        world_state.filtered_game_controller_state.as_ref(),
        field_dimensions,
    );
    let defend_pose = block_on_circle(ball.ball_in_field, position_to_defend, distance_to_target);
    Some(ground_to_field.inverse() * defend_pose)
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
            ground_to_field.inverse()
                * Pose2::<Field>::new(
                    passive_position_to_defend,
                    passive_position_to_defend
                        .look_at(&ball.ball_in_field)
                        .angle(),
                ),
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

fn defend_kick_off_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositionsParameters,
) -> Option<Pose2<Ground>> {
    let ground_to_field = world_state.robot.ground_to_field?;
    let absolute_ball_position = match world_state.ball {
        Some(ball) => ball.ball_in_field,
        None => Point2::origin(),
    };
    let position_to_defend = point![-field_dimensions.length / 2.0, 0.0];
    let center_circle_radius = field_dimensions.center_circle_diameter / 2.0;
    let distance_to_target = distance(position_to_defend, absolute_ball_position)
        - center_circle_radius
        - role_positions.striker_distance_to_non_free_center_circle;
    let defend_pose = block_on_circle(
        absolute_ball_position,
        position_to_defend,
        distance_to_target,
    );
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
