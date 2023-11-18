use std::{
    collections::HashMap,
    f32::consts::FRAC_PI_4,
    mem::take,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use coordinate_systems::Head;
use geometry::line_segment::LineSegment;
use linear_algebra::{vector, Isometry2, Orientation2, Point2, Rotation2, Vector2};
use spl_network_messages::{GamePhase, GameState, HulkMessage, PlayerNumber, Team};
use types::{
    ball_position::{BallPosition, SimulatorBallState},
    game_controller_state::GameControllerState,
    messages::OutgoingMessage,
    motion_command::{HeadMotion, KickVariant, MotionCommand, OrientationMode},
    planned_path::PathSegment,
    players::Players,
    support_foot::Side,
};

use crate::robot::Robot;

pub struct State {
    pub time_elapsed: Duration,
    pub cycle_count: usize,
    pub robots: HashMap<PlayerNumber, Robot>,
    pub ball: Option<SimulatorBallState>,
    pub messages: Vec<(PlayerNumber, HulkMessage)>,
    pub finished: bool,
    pub game_controller_state: GameControllerState,
}

impl State {
    pub fn cycle(&mut self, time_step: Duration) -> Result<()> {
        let now = UNIX_EPOCH + self.time_elapsed;

        self.move_robots(time_step);
        self.cycle_robots(now)?;
        self.move_ball(time_step);

        self.time_elapsed += time_step;
        self.cycle_count += 1;

        Ok(())
    }

    fn move_robots(&mut self, time_step: Duration) {
        for robot in self.robots.values_mut() {
            let ground_to_field = robot
                .database
                .main_outputs
                .ground_to_field
                .as_mut()
                .expect("simulated robots should always have a known pose");

            let head_motion = match &robot.database.main_outputs.motion_command {
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
                        OrientationMode::Override(orientation) => *orientation,
                    };

                    let previous_ground_to_field = *ground_to_field;

                    *ground_to_field = Isometry2::from_parts(
                        (*ground_to_field * step.as_point()).coords(),
                        ground_to_field.orientation().angle()
                            + orientation.angle().clamp(
                                -FRAC_PI_4 * time_step.as_secs_f32(),
                                FRAC_PI_4 * time_step.as_secs_f32(),
                            ),
                    );

                    for obstacle in robot.database.main_outputs.obstacles.iter_mut() {
                        obstacle.position = ground_to_field.inverse()
                            * previous_ground_to_field
                            * obstacle.position;
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
                    if let Some(ball) = self.ball.as_mut() {
                        let side = match kicking_side {
                            Side::Left => -1.0,
                            Side::Right => 1.0,
                        };

                        // TODO: Check if ball is even in range
                        // let kick_location = ground_to_field * ();
                        if (self.time_elapsed - robot.last_kick_time).as_secs_f32() > 1.0 {
                            let direction = match kick {
                                KickVariant::Forward => vector![1.0, 0.0],
                                KickVariant::Turn => vector![0.707, 0.707 * side],
                                KickVariant::Side => vector![0.0, 1.0 * -side],
                            };
                            ball.velocity += *ground_to_field * direction * *strength * 2.5;
                            robot.last_kick_time = self.time_elapsed;
                        };
                    }
                    head
                }
                MotionCommand::SitDown { head } => head,
                MotionCommand::Stand { head } => head,
                _ => &HeadMotion::Center,
            };

            let desired_head_yaw = match head_motion {
                HeadMotion::ZeroAngles => 0.0,
                HeadMotion::Center => 0.0,
                HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                    robot.database.main_outputs.look_around.yaw
                }
                HeadMotion::LookAt { target, .. } => target.coords().angle(Vector2::x_axis()),
                HeadMotion::LookLeftAndRightOf { target } => {
                    let glance_factor = self.time_elapsed.as_secs_f32().sin();
                    target.coords().angle(Vector2::x_axis())
                        + glance_factor * robot.parameters.look_at.glance_angle
                }
                HeadMotion::Unstiff => 0.0,
                HeadMotion::Animation { .. } => 0.0,
            };

            let max_head_rotation_per_cycle =
                robot.parameters.head_motion.maximum_velocity.yaw * time_step.as_secs_f32();
            let diff =
                desired_head_yaw - robot.database.main_outputs.sensor_data.positions.head.yaw;
            let movement = diff.clamp(-max_head_rotation_per_cycle, max_head_rotation_per_cycle);

            robot.database.main_outputs.sensor_data.positions.head.yaw += movement;
        }
    }

    fn cycle_robots(&mut self, now: std::time::SystemTime) -> Result<()> {
        let messages_sent_last_cycle = take(&mut self.messages);

        for (player_number, robot) in self.robots.iter_mut() {
            robot.database.main_outputs.cycle_time.start_time = now;

            let ground_to_field = robot
                .database
                .main_outputs
                .ground_to_field
                .expect("simulated robots should always have a known pose");
            let ball_visible = self.ball.as_ref().is_some_and(|ball| {
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
                    self.ball.as_ref().map(|ball| BallPosition {
                        position: ground_to_field.inverse() * ball.position,
                        velocity: ground_to_field.inverse() * ball.velocity,
                        last_seen: now,
                    })
                } else {
                    None
                };
            robot.database.main_outputs.game_controller_state = Some(self.game_controller_state);
            robot.cycler.cycler_state.ground_to_field = ground_to_field;
            robot.cycle(&messages_sent_last_cycle)?;

            for message in robot.interface.take_outgoing_messages() {
                if let OutgoingMessage::Spl(message) = message {
                    self.messages.push((*player_number, message));
                    self.game_controller_state.remaining_amount_of_messages -= 1
                }
            }
        }

        Ok(())
    }

    fn move_ball(&mut self, time_step: Duration) {
        if let Some(ball) = self.ball.as_mut() {
            ball.position += ball.velocity * time_step.as_secs_f32();
            ball.velocity *= 0.98;
        }
    }

    pub fn spawn_robot(&mut self, player_number: PlayerNumber) -> Result<&mut Robot> {
        self.robots
            .insert(player_number, Robot::try_new(player_number)?);

        Ok(self
            .robots
            .get_mut(&player_number)
            .expect("robot was inserted just now and vanished"))
    }
}

impl Default for State {
    fn default() -> Self {
        let robots = HashMap::new();
        let game_controller_state = GameControllerState {
            game_state: GameState::Initial,
            game_phase: GamePhase::Normal,
            kicking_team: Team::Hulks,
            last_game_state_change: SystemTime::UNIX_EPOCH,
            penalties: Players {
                one: None,
                two: None,
                three: None,
                four: None,
                five: None,
                six: None,
                seven: None,
            },
            opponent_penalties: Players {
                one: None,
                two: None,
                three: None,
                four: None,
                five: None,
                six: None,
                seven: None,
            },
            remaining_amount_of_messages: 1200,
            sub_state: None,
            hulks_team_is_home_after_coin_toss: true,
        };

        Self {
            time_elapsed: Duration::ZERO,
            cycle_count: 0,
            robots,
            ball: None,
            messages: Vec::new(),
            finished: false,
            game_controller_state,
        }
    }
}
