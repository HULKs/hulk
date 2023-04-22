use std::ops::Range;

use framework::AdditionalOutput;
use nalgebra::{distance, point, Isometry2, Point2};
use spl_network_messages::{GamePhase, SubState, Team};
use types::{
    configuration::RolePositions, rotate_towards, BallState, FieldDimensions, GameControllerState,
    Line, MotionCommand, PathObstacle, Side, WorldState,
};

use super::{head::LookAction, walk_to_pose::WalkAndStand};

pub struct Defend<'cycle> {
    world_state: &'cycle WorldState,
    field_dimensions: &'cycle FieldDimensions,
    role_positions: &'cycle RolePositions,
    walk_and_stand: &'cycle WalkAndStand<'cycle>,
    look_action: &'cycle LookAction<'cycle>,
}

impl<'cycle> Defend<'cycle> {
    pub fn new(
        world_state: &'cycle WorldState,
        field_dimensions: &'cycle FieldDimensions,
        role_positions: &'cycle RolePositions,
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
        pose: Isometry2<f32>,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        self.walk_and_stand
            .execute(pose, self.look_action.execute(), path_obstacles_output)
    }

    pub fn left(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_left_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn penalty_left(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose =
            defend_penalty_left_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn right(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_right_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn penalty_right(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_penalty_right_pose(
            self.world_state,
            self.field_dimensions,
            self.role_positions,
        )?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn penalty_kick(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose =
            defend_penalty_kick(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn goal(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose = defend_goal_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }

    pub fn kick_off(
        &self,
        path_obstacles_output: &mut AdditionalOutput<Vec<PathObstacle>>,
    ) -> Option<MotionCommand> {
        let pose =
            defend_kick_off_pose(self.world_state, self.field_dimensions, self.role_positions)?;
        self.with_pose(pose, path_obstacles_output)
    }
}

fn defend_left_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

fn defend_penalty_left_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let position_to_defend = point![
        (-field_dimensions.length + field_dimensions.penalty_area_length) / 2.0,
        role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

fn defend_right_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let position_to_defend = point![
        -field_dimensions.length / 2.0,
        -role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Right {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}
fn defend_penalty_right_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let position_to_defend = point![
        (-field_dimensions.length + field_dimensions.penalty_area_length) / 2.0,
        -role_positions.defender_y_offset
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

fn defend_penalty_kick(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: point![
                -field_dimensions.length / 2.0 + field_dimensions.penalty_marker_distance,
                0.0
            ],
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let position_to_defend = point![
        (-field_dimensions.length + field_dimensions.penalty_area_length) / 2.0,
        0.0
    ];
    let distance_to_target = if ball.field_side == Side::Left {
        role_positions.defender_aggressive_ring_radius
    } else {
        role_positions.defender_passive_ring_radius
    };

    let defend_pose = block_on_circle(ball.position, position_to_defend, distance_to_target);
    Some(robot_to_field.inverse() * defend_pose)
}

fn defend_goal_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let ball = world_state
        .ball
        .map(|ball| BallState {
            position: robot_to_field * ball.position,
            field_side: ball.field_side,
            penalty_shot_direction: Default::default(),
        })
        .unwrap_or_default();

    let keeper_x_offset = match world_state.game_controller_state {
        Some(GameControllerState {
            game_phase:
                GamePhase::PenaltyShootout {
                    kicking_team: Team::Opponent,
                },
            ..
        })
        | Some(GameControllerState {
            sub_state: Some(SubState::PenaltyKick),
            kicking_team: Team::Opponent,
            ..
        }) => 0.0,
        _ => role_positions.keeper_x_offset,
    };

    let position_to_defend = point![-field_dimensions.length / 2.0 - 1.0, 0.0];
    let defend_pose = block_on_line(
        ball.position,
        position_to_defend,
        -field_dimensions.length / 2.0 + keeper_x_offset,
        -0.7..0.7,
    );
    Some(robot_to_field.inverse() * defend_pose)
}

fn defend_kick_off_pose(
    world_state: &WorldState,
    field_dimensions: &FieldDimensions,
    role_positions: &RolePositions,
) -> Option<Isometry2<f32>> {
    let robot_to_field = world_state.robot.robot_to_field?;
    let absolute_ball_position = match world_state.ball {
        Some(ball) => robot_to_field * ball.position,
        None => Point2::origin(),
    };
    let position_to_defend = point![-field_dimensions.length / 2.0, 0.0];
    let distance_to_target = distance(&position_to_defend, &absolute_ball_position)
        - role_positions.striker_distance_to_non_free_ball;
    let defend_pose = block_on_circle(
        absolute_ball_position,
        position_to_defend,
        distance_to_target,
    );
    Some(robot_to_field.inverse() * defend_pose)
}

pub fn block_on_circle(
    ball_position: Point2<f32>,
    target: Point2<f32>,
    distance_to_target: f32,
) -> Isometry2<f32> {
    let target_to_ball = ball_position - target;
    let block_position = target + target_to_ball.normalize() * distance_to_target;
    Isometry2::new(
        block_position.coords,
        rotate_towards(block_position, ball_position).angle(),
    )
}

fn block_on_line(
    ball_position: Point2<f32>,
    target: Point2<f32>,
    defense_line_x: f32,
    defense_line_y_range: Range<f32>,
) -> Isometry2<f32> {
    let is_ball_in_front_of_defense_line = defense_line_x < ball_position.x;
    if is_ball_in_front_of_defense_line {
        let defense_line = Line(
            point![defense_line_x, defense_line_y_range.start],
            point![defense_line_x, defense_line_y_range.end],
        );
        let ball_target_line = Line(ball_position, target);
        let intersection_point = defense_line.intersection(&ball_target_line);
        let defense_position = point![
            intersection_point.x,
            intersection_point
                .y
                .clamp(defense_line_y_range.start, defense_line_y_range.end)
        ];
        Isometry2::new(
            defense_position.coords,
            rotate_towards(defense_position, ball_position).angle(),
        )
    } else {
        let defense_position = point![
            defense_line_x,
            (defense_line_y_range.start + defense_line_y_range.end) / 2.0
        ];
        Isometry2::new(
            defense_position.coords,
            rotate_towards(defense_position, ball_position).angle(),
        )
    }
}
