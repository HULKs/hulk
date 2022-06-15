use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use macros::{module, require_some};

use anyhow::Result;
use approx::RelativeEq;
use nalgebra::{point, vector, Isometry2, Point2, Rotation2, Vector2};
use spl_network::{GameControllerReturnMessage, SplMessage};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::{
    framework::configuration::SetPositions,
    types::{
        BallPosition, FallState, FieldDimensions, GroundContact, MessageReceivers, PrimaryState,
        Role, SensorData, WorldState,
    },
};

pub struct WorldStateComposer {
    last_transmitted_messages: Option<SystemTime>,
    game_controller_return_message_sender: UnboundedSender<GameControllerReturnMessage>,
    game_controller_return_message_receiver:
        Arc<Mutex<UnboundedReceiver<GameControllerReturnMessage>>>,
    _spl_message_sender: UnboundedSender<SplMessage>,
    spl_message_receiver: Arc<Mutex<UnboundedReceiver<SplMessage>>>,
    world_state: WorldState,
}

#[module(control)]
#[input(path = ball_position, data_type = BallPosition)]
#[input(path = fall_state, data_type = FallState)]
#[input(path = robot_to_field, data_type = Isometry2<f32>)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = primary_state, data_type = PrimaryState)]
#[input(path = ground_contact, data_type = GroundContact)]
#[perception_input(path = spl_message, data_type = SplMessage, cycler = spl_network)]
#[parameter(path = control.set_positions, data_type = SetPositions)]
#[parameter(path = field_dimensions, data_type = FieldDimensions)]
#[parameter(path = player_number, data_type = usize)]
#[main_output(data_type = MessageReceivers)]
#[main_output(data_type = WorldState)]
impl WorldStateComposer {}

impl WorldStateComposer {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        let (game_controller_return_message_sender, game_controller_return_message_receiver) =
            unbounded_channel();
        let (_spl_message_sender, spl_message_receiver) = unbounded_channel();
        Ok(Self {
            last_transmitted_messages: None,
            game_controller_return_message_sender,
            game_controller_return_message_receiver: Arc::new(Mutex::new(
                game_controller_return_message_receiver,
            )),
            _spl_message_sender,
            spl_message_receiver: Arc::new(Mutex::new(spl_message_receiver)),
            world_state: WorldState::new(),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;
        let ball = require_some!(context.ball_position);
        let fall_state = require_some!(context.fall_state);
        let pose = *require_some!(context.robot_to_field);
        let primary_state = require_some!(context.primary_state);
        let ground_contact = require_some!(context.ground_contact);

        let role = match context.player_number {
            1 => Role::Keeper,
            2 => Role::DefenderRight,
            3 => Role::DefenderLeft,
            4 => Role::DefenderFront,
            _ => Role::Striker,
        };
        let walk_target_pose = create_walk_target(
            &role,
            ball,
            pose,
            context.set_positions,
            context.field_dimensions,
            *primary_state,
        );

        self.world_state.ball = *ball;
        self.world_state.robot.role = role;
        self.world_state.robot.fall_state = *fall_state;
        self.world_state.robot.pose = pose;
        self.world_state.robot.primary_state = *primary_state;
        self.world_state.robot.walk_target_pose = walk_target_pose.unwrap_or_default();
        self.world_state.robot.has_ground_contact = ground_contact.any_foot();

        if self.last_transmitted_messages.is_none()
            || cycle_start_time.duration_since(self.last_transmitted_messages.unwrap())?
                > Duration::from_secs(1)
        {
            self.last_transmitted_messages = Some(cycle_start_time);
            self.game_controller_return_message_sender
                .send(GameControllerReturnMessage {
                    player_number: *context.player_number as u8,
                    fallen: matches!(self.world_state.robot.fall_state, FallState::Fallen { .. }),
                    robot_to_field: self.world_state.robot.pose,
                    ball_position: None,
                })?;
        }

        Ok(MainOutputs {
            world_state: Some(self.world_state.clone()),
            message_receivers: Some(MessageReceivers {
                game_controller_return_message_receiver: self
                    .game_controller_return_message_receiver
                    .clone(),
                spl_message_receiver: self.spl_message_receiver.clone(),
            }),
        })
    }
}

fn create_walk_target(
    role: &Role,
    ball: &BallPosition,
    current_pose: Isometry2<f32>,
    set_positions: &SetPositions,
    field_dimensions: &FieldDimensions,
    primary_state: PrimaryState,
) -> Option<Isometry2<f32>> {
    let own_goal_center = vector!(-field_dimensions.length * 0.5, 0.0);
    match (primary_state, role, ball.position) {
        (PrimaryState::Initial, _, _) => Some(current_pose),
        (PrimaryState::Ready, Role::DefenderFront, _)
        | (PrimaryState::Playing, Role::DefenderFront, None) => Some(Isometry2::new(
            own_goal_center + set_positions.defender_front_set_position_goal_center_offset,
            0.0,
        )),
        (PrimaryState::Ready, Role::DefenderLeft, _)
        | (PrimaryState::Playing, Role::DefenderLeft, None) => Some(Isometry2::new(
            own_goal_center + set_positions.defender_left_set_position_goal_center_offset,
            0.0,
        )),
        (PrimaryState::Ready, Role::DefenderRight, _)
        | (PrimaryState::Playing, Role::DefenderRight, None) => Some(Isometry2::new(
            own_goal_center + set_positions.defender_right_set_position_goal_center_offset,
            0.0,
        )),
        (PrimaryState::Ready, Role::Keeper, _) | (PrimaryState::Playing, Role::Keeper, None) => {
            Some(Isometry2::new(
                own_goal_center + set_positions.keeper_set_position_goal_center_offset,
                0.0,
            ))
        }
        (PrimaryState::Ready, Role::Striker, _) => {
            Some(Isometry2::new(set_positions.striker_set_position, 0.0))
        }
        (PrimaryState::Set, _, _) => Some(current_pose),
        (PrimaryState::Playing, Role::DefenderFront, Some(ball_position)) => Some(
            create_dribble_pose(current_pose, ball_position, field_dimensions.length),
        ),
        (PrimaryState::Playing, Role::DefenderLeft, Some(ball_position)) => Some(
            create_dribble_pose(current_pose, ball_position, field_dimensions.length),
        ),
        (PrimaryState::Playing, Role::DefenderRight, Some(ball_position)) => Some(
            create_dribble_pose(current_pose, ball_position, field_dimensions.length),
        ),
        (PrimaryState::Playing, Role::Keeper, Some(ball_position)) => Some(Isometry2::new(
            create_blocker_translation(
                (own_goal_center + set_positions.keeper_set_position_goal_center_offset).x,
                -0.7,
                0.7,
                current_pose * ball_position,
                point!(-field_dimensions.length * 0.5, 0.75),
                point!(-field_dimensions.length * 0.5, -0.75),
            ),
            0.0,
        )),
        (PrimaryState::Playing, Role::Striker, None) => {
            Some(current_pose * Isometry2::rotation(1.0))
        }
        (PrimaryState::Playing, Role::Striker, Some(ball_position)) => Some(create_dribble_pose(
            current_pose,
            ball_position,
            field_dimensions.length,
        )),
        (PrimaryState::Finished, _, _) => Some(current_pose),
        (_, _, _) => None,
    }
}

fn create_dribble_pose(
    current_pose: Isometry2<f32>,
    ball_position: Point2<f32>,
    field_length: f32,
) -> Isometry2<f32> {
    let absolute_ball_position = current_pose * ball_position;
    let opponent_goal_center = point!(field_length * 0.5, 0.0);
    let ball_towards_opponent_goal_center =
        (opponent_goal_center - absolute_ball_position).try_normalize(0.0);
    match ball_towards_opponent_goal_center {
        None => current_pose,
        Some(direction_from_ball_to_opponent_goal_center) => {
            let target_position_to_start_dribble =
                absolute_ball_position - 0.15 * direction_from_ball_to_opponent_goal_center;
            let dribble_start_pose = Isometry2::new(
                target_position_to_start_dribble.coords,
                Rotation2::rotation_between(
                    &vector!(1.0, 0.0),
                    &direction_from_ball_to_opponent_goal_center,
                )
                .angle(),
            );
            if current_pose
                .translation
                .relative_eq(&dribble_start_pose.translation, 0.05, 0.05)
                && current_pose.rotation.angle().relative_eq(
                    &dribble_start_pose.rotation.angle(),
                    0.4,
                    0.4,
                )
            {
                Isometry2::new(
                    (dribble_start_pose * point!(1.0, 0.0)).coords,
                    dribble_start_pose.rotation.angle(),
                )
            } else {
                dribble_start_pose
            }
        }
    }
}

fn create_blocker_translation(
    defense_line_x: f32,
    defense_line_minimum_y: f32,
    defense_line_maximum_y: f32,
    absolute_ball_position: Point2<f32>,
    absolute_left_goal_post_position: Point2<f32>,
    absolute_right_goal_post_position: Point2<f32>,
) -> Vector2<f32> {
    if defense_line_x < absolute_ball_position.x {
        let translation_to_put_origin_at_defense_x_ball_y =
            vector!(-defense_line_x, -absolute_ball_position.y);

        let ball_in_convenient_coordinates =
            absolute_ball_position + translation_to_put_origin_at_defense_x_ball_y;
        let left_goal_post_in_convenient_coordinates =
            absolute_left_goal_post_position + translation_to_put_origin_at_defense_x_ball_y;
        let right_goal_post_in_convenient_coordinates =
            absolute_right_goal_post_position + translation_to_put_origin_at_defense_x_ball_y;

        let slope_between_left_goal_post_and_ball = left_goal_post_in_convenient_coordinates.y
            / (ball_in_convenient_coordinates.x - left_goal_post_in_convenient_coordinates.x);
        let slope_between_right_goal_post_and_ball = right_goal_post_in_convenient_coordinates.y
            / (ball_in_convenient_coordinates.x - right_goal_post_in_convenient_coordinates.x);

        let left_goal_post_intersection =
            ball_in_convenient_coordinates.x * slope_between_left_goal_post_and_ball;
        let right_goal_post_intersection =
            ball_in_convenient_coordinates.x * slope_between_right_goal_post_and_ball;
        let center_of_intersection = point!(
            0.0,
            (left_goal_post_intersection + right_goal_post_intersection) * 0.5
        );

        let absolute_center_of_intersection =
            center_of_intersection - translation_to_put_origin_at_defense_x_ball_y;

        vector!(
            defense_line_x,
            absolute_center_of_intersection
                .y
                .clamp(defense_line_minimum_y, defense_line_maximum_y)
        )
    } else {
        vector!(
            defense_line_x,
            (defense_line_minimum_y + defense_line_maximum_y) * 0.5
        )
    }
}
