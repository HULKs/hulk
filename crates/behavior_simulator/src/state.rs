use nalgebra::{point, vector, Isometry2, Point2, Translation2, UnitComplex, Vector2};
use std::time::{Duration, UNIX_EPOCH};
use types::{LineSegment, MotionCommand, PathSegment, PrimaryState};

use crate::robot::Robot;

pub struct State {
    pub time_elapsed: Duration,
    pub robots: Vec<Robot>,
    pub ball: Option<Point2<f32>>,
    pub ball_velocity: Vector2<f32>,
}

impl State {
    pub fn new(robot_count: usize) -> Self {
        let robots: Vec<_> = (0..robot_count).map(Robot::new).collect();

        Self {
            time_elapsed: Duration::ZERO,
            robots,
            ball: None,
            ball_velocity: Vector2::new(0.0, 1.0),
        }
    }

    pub fn stiffen_robots(&mut self) {
        for robot in &mut self.robots {
            robot.database.main_outputs.primary_state = PrimaryState::Playing;
        }
    }

    pub fn cycle(&mut self, time_step: Duration) {
        let now = UNIX_EPOCH + self.time_elapsed;

        for robot in &mut self.robots {
            let robot_to_field = robot.database.main_outputs.robot_to_field.unwrap();

            if self.ball.is_none() && self.time_elapsed.as_secs_f32() > 6.0 {
                self.ball = Some(point![1.0, 0.0]);
            }

            if let Some(position) = self.ball {
                robot.database.main_outputs.ball_position = Some(types::BallPosition {
                    position: robot_to_field.inverse() * position,
                    last_seen: now,
                })
            }

            robot.cycle().unwrap();

            let robot_to_field = robot
                .database
                .main_outputs
                .robot_to_field
                .as_mut()
                .expect("Simulated robots should always have a known pose");
            match &robot.database.main_outputs.motion_command {
                MotionCommand::Walk {
                    path,
                    orientation_mode,
                    ..
                } => {
                    let step = robot_to_field.rotation
                        * match path[0] {
                            PathSegment::LineSegment(LineSegment(_start, end)) => end,
                            PathSegment::Arc(arc, _orientation) => arc.end,
                        }
                        .coords
                        .cap_magnitude(0.3 * time_step.as_secs_f32());
                    robot_to_field.append_translation_mut(&Translation2::new(step.x, step.y));
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
                    robot_to_field.append_rotation_wrt_center_mut(&orientation);
                    *robot_to_field = Isometry2::new(
                        robot_to_field.translation.vector,
                        robot_to_field.rotation.angle()
                            + (orientation.angle())
                                .clamp(0.0, std::f32::consts::FRAC_PI_2 * time_step.as_secs_f32()),
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
        }
        if let Some(ball) = self.ball.as_mut() {
            *ball += self.ball_velocity * time_step.as_secs_f32();
            self.ball_velocity *= 0.98;

            if ball.x.abs() > 4.5 && ball.y < 0.75 {
                *ball = Point2::origin();
            }
        }
        self.time_elapsed += time_step;
    }
}
