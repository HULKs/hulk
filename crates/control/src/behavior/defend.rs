use std::ops::Range;

use coordinate_systems::{Field, Ground};
use filtering::hysteresis::greater_than_with_hysteresis;
use framework::AdditionalOutput;
use geometry::{
    line::{Line, Line2},
    look_at::LookAt,
};
use hsl_network_messages::{GamePhase, SubState, Team};
use linear_algebra::{distance, point, Point2, Pose2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{JumpDirection, MotionCommand, OrientationMode, WalkSpeed},
    parameters::{KeeperMotionParameters, RolePositionsParameters},
    path_obstacles::PathObstacle,
    world_state::{BallState, WorldState},
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefendMode {
    Aggressive,
    Passive,
}

pub struct Defend<'cycle> {
    world_state: &'cycle WorldState,
    field_dimensions: &'cycle FieldDimensions,
    role_positions: &'cycle RolePositionsParameters,
    walk_and_stand: &'cycle WalkAndStand<'cycle>,
    look_action: &'cycle LookAction<'cycle>,
    last_defender_mode: &'cycle mut DefendMode,
}

impl<'cycle> Defend<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        role_positions: &'cycle RolePositionsParameters,
        walk_and_stand: &'cycle WalkAndStand,
        look_action: &'cycle LookAction,
        last_defender_mode: &'cycle mut DefendMode,
    ) -> Self {
        Self {
            world_state,
            field_dimensions,
            role_positions,
            walk_and_stand,
            look_action,
            last_defender_mode,
        }
    }

    fn with_pose(
        &self,
        pose: Pose2<Ground>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
        walk_speed: WalkSpeed,
        distance_to_be_aligned: f32,
        hysteresis: nalgebra::Vector2<f32>,
    ) -> Option<MotionCommand> {
        self.walk_and_stand.execute(
            pose,
            self.look_action.execute(),
            path_obstacles_output,
            walk_speed,
            // TODO(rmburg): maybe change this instead of having a large distance_to_be_aligned?
            OrientationMode::AlignWithPath,
            distance_to_be_aligned,
            hysteresis,
        )
    }

    pub fn keeper_motion(&self, parameters: KeeperMotionParameters) -> Option<MotionCommand> {
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

        if (-parameters.action_radius_center..=parameters.action_radius_center)
            .contains(&horizontal_distance_to_intersection)
        {
            Some(MotionCommand::KeeperMotion {
                direction: JumpDirection::Center,
            })
        } else if (parameters.action_radius_center..parameters.action_radius_left)
            .contains(&horizontal_distance_to_intersection)
        {
            Some(MotionCommand::KeeperMotion {
                direction: JumpDirection::Left,
            })
        } else if (-parameters.action_radius_left..-parameters.action_radius_center)
            .contains(&horizontal_distance_to_intersection)
        {
            Some(MotionCommand::KeeperMotion {
                direction: JumpDirection::Right,
            })
        } else {
            None
        }
    }

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

    pub fn right(
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
            Side::Right,
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

    pub fn opponent_corner_kick(
        &mut self,
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
            self.walk_and_stand.parameters.defender_hysteresis,
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
            self.walk_and_stand.parameters.hysteresis,
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
            self.walk_and_stand.parameters.defender_hysteresis,
        )
    }
}

fn defend_pose(
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
