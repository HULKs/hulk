use std::{
    f32::consts::FRAC_PI_4,
    mem::take,
    time::{Duration, SystemTime},
};

use bevy::{
    ecs::{
        event::Event,
        system::{Query, Res, ResMut, Resource},
    },
    time::Time,
};

use ball_filter::BallPosition;
use coordinate_systems::{Field, Ground, Head};
use geometry::line_segment::LineSegment;
use linear_algebra::{vector, Isometry2, Orientation2, Point2, Rotation2, Vector2};
use spl_network_messages::{GameState, HulkMessage, PlayerNumber};
use types::{
    messages::OutgoingMessage,
    motion_command::{HeadMotion, KickVariant, MotionCommand, OrientationMode},
    planned_path::PathSegment,
    primary_state::PrimaryState,
    support_foot::Side,
};

use crate::{ball::BallResource, game_controller::GameController, robot::Robot};

pub fn move_robots(mut robots: Query<&mut Robot>, mut ball: ResMut<BallResource>, time: Res<Time>) {
    let time_step = Duration::from_secs_f32(0.012);
    for mut robot in &mut robots {
        let mut new_ground_to_field: Option<Isometry2<Ground, Field>> = None;
        let ground_to_field = robot
            .database
            .main_outputs
            .ground_to_field
            .expect("simulated robots should always have a known pose");

        let head_motion = match robot.database.main_outputs.motion_command.clone() {
            MotionCommand::Walk {
                head,
                path,
                orientation_mode,
                ..
            } => {
                let step = match path[0] {
                    PathSegment::LineSegment(LineSegment(_start, end)) => end.coords(),
                    PathSegment::Arc(arc, direction) => {
                        direction.rotate_vector_90_degrees(arc.start - arc.circle.center)
                    }
                }
                .cap_magnitude(0.3 * time_step.as_secs_f32());

                let orientation = match orientation_mode {
                    OrientationMode::AlignWithPath => {
                        if step.norm_squared() < f32::EPSILON {
                            Orientation2::identity()
                        } else {
                            Orientation2::from_vector(step)
                        }
                    }
                    OrientationMode::Override(orientation) => orientation,
                };

                let previous_ground_to_field = ground_to_field;

                new_ground_to_field = Some(Isometry2::from_parts(
                    (ground_to_field * step.as_point()).coords(),
                    ground_to_field.orientation().angle()
                        + orientation.angle().clamp(
                            -FRAC_PI_4 * time_step.as_secs_f32(),
                            FRAC_PI_4 * time_step.as_secs_f32(),
                        ),
                ));

                for obstacle in robot.database.main_outputs.obstacles.iter_mut() {
                    obstacle.position =
                        ground_to_field.inverse() * previous_ground_to_field * obstacle.position;
                }

                head
            }
            MotionCommand::InWalkKick {
                head,
                kick,
                kicking_side,
                strength,
                ..
            } => {
                if let Some(ball) = ball.state.as_mut() {
                    let side = match kicking_side {
                        Side::Left => -1.0,
                        Side::Right => 1.0,
                    };

                    // TODO: Check if ball is even in range
                    // let kick_location = ground_to_field * ();
                    if dbg!((time.elapsed() - robot.last_kick_time).as_secs_f32()) > 1.0 {
                        let direction = match kick {
                            KickVariant::Forward => vector![1.0, 0.0],
                            KickVariant::Turn => vector![0.707, 0.707 * side],
                            KickVariant::Side => vector![0.0, 1.0 * -side],
                        };
                        ball.velocity += ground_to_field * direction * strength * 2.5;
                        robot.last_kick_time = time.elapsed();
                    };
                }
                head
            }
            MotionCommand::SitDown { head } => head,
            MotionCommand::Stand { head } => head,
            _ => HeadMotion::Center,
        };

        let desired_head_yaw = match head_motion {
            HeadMotion::ZeroAngles => 0.0,
            HeadMotion::Center => 0.0,
            HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                robot.database.main_outputs.look_around.yaw
            }
            HeadMotion::LookAt { target, .. } => target.coords().angle(Vector2::x_axis()),
            HeadMotion::LookLeftAndRightOf { target } => {
                let glance_factor = 0.0; //self.time_elapsed.as_secs_f32().sin();
                target.coords().angle(Vector2::x_axis())
                    + glance_factor * robot.parameters.look_at.glance_angle
            }
            HeadMotion::Unstiff => 0.0,
        };

        let max_head_rotation_per_cycle =
            robot.parameters.head_motion.maximum_velocity.yaw * time_step.as_secs_f32();
        let diff = desired_head_yaw - robot.database.main_outputs.sensor_data.positions.head.yaw;
        let movement = diff.clamp(-max_head_rotation_per_cycle, max_head_rotation_per_cycle);

        robot.database.main_outputs.sensor_data.positions.head.yaw += movement;
        if let Some(new_ground_to_field) = new_ground_to_field {
            robot.database.main_outputs.ground_to_field = Some(new_ground_to_field);
        }
    }
}

#[derive(Event, Clone, Copy)]
pub struct Message {
    pub sender: PlayerNumber,
    pub payload: HulkMessage,
}

#[derive(Resource, Default)]
pub struct Messages {
    pub messages: Vec<Message>,
}

pub fn cycle_robots(
    mut robots: Query<&mut Robot>,
    ball: Res<BallResource>,
    mut game_controller: ResMut<GameController>,
    time: Res<Time>,
    mut messages: ResMut<Messages>,
) {
    let messages_sent_last_cycle = take(&mut messages.messages);
    let now = SystemTime::UNIX_EPOCH + time.elapsed();

    for mut robot in &mut robots {
        robot.database.main_outputs.cycle_time.start_time = now;

        let ground_to_field = robot
            .database
            .main_outputs
            .ground_to_field
            .expect("simulated robots should always have a known pose");
        let ball_visible = ball.state.as_ref().is_some_and(|ball| {
            let ball_in_ground = ground_to_field.inverse() * ball.position;
            let head_to_ground =
                Rotation2::new(robot.database.main_outputs.sensor_data.positions.head.yaw);
            let ball_in_head: Point2<Head> = head_to_ground.inverse() * ball_in_ground;
            let field_of_view = robot.field_of_view();
            let angle_to_ball = ball_in_head.coords().angle(Vector2::x_axis());

            angle_to_ball.abs() < field_of_view / 2.0 && ball_in_head.coords().norm() < 3.0
        });
        if ball_visible {
            robot.ball_last_seen = Some(now);
        }
        robot.database.main_outputs.ball_position =
            if robot.ball_last_seen.is_some_and(|last_seen| {
                now.duration_since(last_seen).expect("time ran backwards")
                    < robot.parameters.ball_filter.hypothesis_timeout
            }) {
                ball.state.as_ref().map(|ball| BallPosition {
                    position: ground_to_field.inverse() * ball.position,
                    velocity: ground_to_field.inverse() * ball.velocity,
                    last_seen: now,
                })
            } else {
                None
            };
        robot.database.main_outputs.primary_state =
            match (robot.is_penalized, game_controller.state.game_state) {
                (true, _) => PrimaryState::Penalized,
                (false, GameState::Initial) => PrimaryState::Initial,
                (false, GameState::Standby) => PrimaryState::Standby,
                (false, GameState::Ready { .. }) => PrimaryState::Ready,
                (false, GameState::Set) => PrimaryState::Set,
                (false, GameState::Playing { .. }) => PrimaryState::Playing,
                (false, GameState::Finished) => PrimaryState::Finished,
            };
        robot.database.main_outputs.game_controller_state = Some(game_controller.state);
        robot.cycle(&messages_sent_last_cycle).unwrap();

        for message in robot.interface.take_outgoing_messages() {
            if let OutgoingMessage::Spl(message) = message {
                messages.messages.push(Message {
                    sender: robot.parameters.player_number,
                    payload: message,
                });
                game_controller.state.remaining_amount_of_messages -= 1
            }
        }
    }
}
