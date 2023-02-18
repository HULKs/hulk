use mlua::{Function, Lua};
use nalgebra::{point, vector, Isometry2, Point2, UnitComplex, Vector2};
use parking_lot::Mutex;
use spl_network_messages::SplMessage;
use std::{
    collections::BTreeMap,
    iter::once,
    mem::take,
    sync::Arc,
    time::{Duration, UNIX_EPOCH},
};
use structs::control::AdditionalOutputs;
use types::{
    messages::{IncomingMessage, OutgoingMessage},
    LineSegment, MotionCommand, PathSegment, PrimaryState,
};

use crate::robot::Robot;

enum Event {
    Cycle,
    Goal,
}

pub struct InnerState {
    pub time_elapsed: Duration,
    pub robots: Vec<Robot>,
    pub ball: Option<Point2<f32>>,
    pub ball_velocity: Vector2<f32>,
    pub messages: Vec<(usize, SplMessage)>,
}

pub struct State {
    pub inner: Arc<Mutex<InnerState>>,
}

impl InnerState {
    pub fn new() -> Self {
        let robots = Vec::new();

        Self {
            time_elapsed: Duration::ZERO,
            robots,
            ball: None,
            ball_velocity: Vector2::new(0.0, 1.0),
            messages: Vec::new(),
        }
    }

    fn cycle(&mut self, time_step: Duration) -> Vec<Event> {
        let now = UNIX_EPOCH + self.time_elapsed;

        let incoming_messages = take(&mut self.messages);

        let mut events = vec![Event::Cycle];

        for (index, robot) in self.robots.iter_mut().enumerate() {
            let robot_to_field = robot
                .database
                .main_outputs
                .robot_to_field
                .as_mut()
                .expect("Simulated robots should always have a known pose");

            robot.database.additional_outputs = AdditionalOutputs::default();
            match &robot.database.main_outputs.motion_command {
                MotionCommand::Walk {
                    path,
                    orientation_mode,
                    ..
                } => {
                    let step = match path[0] {
                        PathSegment::LineSegment(LineSegment(_start, end)) => end,
                        PathSegment::Arc(arc, _orientation) => arc.end,
                    }
                    .coords
                    .cap_magnitude(0.3 * time_step.as_secs_f32());
                    let orientation = match orientation_mode {
                        types::OrientationMode::AlignWithPath => {
                            if step.norm_squared() < f32::EPSILON {
                                UnitComplex::identity()
                            } else {
                                UnitComplex::from_cos_sin_unchecked(step.x, step.y)
                            }
                        }
                        types::OrientationMode::Override(orientation) => *orientation,
                    };

                    *robot_to_field = Isometry2::new(
                        robot_to_field.translation.vector + robot_to_field.rotation * step,
                        robot_to_field.rotation.angle()
                            + orientation.angle().clamp(
                                -std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                                std::f32::consts::FRAC_PI_4 * time_step.as_secs_f32(),
                            ),
                    )
                }
                MotionCommand::InWalkKick {
                    head: _,
                    kick,
                    kicking_side,
                } => {
                    if let Some(_ball) = self.ball {
                        let side = match kicking_side {
                            types::Side::Left => 1.0,
                            types::Side::Right => -1.0,
                        };

                        // TODO: Check if ball is even in range
                        // let kick_location = robot_to_field * ();

                        let strength = 1.0;
                        let direction = match kick {
                            types::KickVariant::Forward => vector![1.0, 0.0],
                            types::KickVariant::Turn => vector![0.707, 0.707 * side],
                            types::KickVariant::Side => vector![0.0, 1.0 * -side],
                        };
                        self.ball_velocity += *robot_to_field * direction * strength;
                    }
                }
                _ => {}
            }

            let incoming_messages: Vec<_> = incoming_messages
                .iter()
                .filter_map(|(sender, message)| {
                    (*sender != index).then_some(IncomingMessage::Spl(*message))
                })
                .collect();
            robot.database.main_outputs.game_controller_state = Some(types::GameControllerState {
                game_state: spl_network_messages::GameState::Playing,
                game_phase: spl_network_messages::GamePhase::Normal,
                kicking_team: spl_network_messages::Team::Uncertain,
                last_game_state_change: now,
                penalties: Default::default(),
                remaining_amount_of_messages: 1200,
                set_play: None,
            });
            let messages = incoming_messages.iter().collect();
            let messages = BTreeMap::from_iter(once((now, messages)));
            if self.ball.is_none() && self.time_elapsed.as_secs_f32() > 6.0 {
                self.ball = Some(point![1.0, 0.0]);
            }
            robot.database.main_outputs.cycle_time.start_time = now;

            if let Some(position) = self.ball {
                robot.database.main_outputs.ball_position = Some(types::BallPosition {
                    position: robot_to_field.inverse() * position,
                    last_seen: now,
                })
            }

            robot.cycle(messages).unwrap();

            for message in robot.interface.take_outgoing_messages() {
                if let OutgoingMessage::Spl(message) = message {
                    self.messages.push((index, message));
                }
            }
        }

        if let Some(ball) = self.ball.as_mut() {
            *ball += self.ball_velocity * time_step.as_secs_f32();
            self.ball_velocity *= 0.98;

            if ball.x.abs() > 4.5 && ball.y < 0.75 {
                events.push(Event::Goal);
            }
        }

        self.time_elapsed += time_step;

        events
    }

    pub fn spawn_robot(&mut self, number: usize) {
        println!("Spawning robot {number}");
        self.robots.push(Robot::new(number));
    }

    pub fn stiffen_robots(&mut self) {
        for robot in &mut self.robots {
            robot.database.main_outputs.primary_state = PrimaryState::Playing;
        }
    }
}

impl State {
    pub fn new() -> Self {
        let inner = Arc::new(Mutex::new(InnerState::new()));

        Self { inner }
    }

    pub fn cycle(&mut self, lua: &Lua) {
        let events = {
            let mut inner = self.inner.lock();
            inner.cycle(Duration::from_millis(12))
        };

        for event in events {
            match event {
                Event::Cycle => {},
                Event::Goal => {
                    if let Ok(on_goal) = lua.globals().get::<_, Function>("on_goal") {
                        on_goal.call::<_, ()>(()).unwrap();
                    }
                }
            }
        }
    }
}

impl mlua::UserData for InnerState {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("spawn_robot", |_, this, number| {
            this.spawn_robot(number);

            Ok(())
        });
        methods.add_method_mut("return_ball_to_center", |_, this, ()| {
            if let Some(ball) = this.ball.as_mut() {
                *ball = Point2::origin();
                this.ball_velocity = Vector2::zeros()
            }
            Ok(())
        });
    }
}
