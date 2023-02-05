use nalgebra::{point, Point2, Translation2};
use std::time::{Duration, UNIX_EPOCH};
use types::{LineSegment, MotionCommand, PathSegment, PrimaryState};

use crate::robot::Robot;

pub struct State {
    pub time_elapsed: Duration,
    pub robots: Vec<Robot>,
    pub ball: Option<Point2<f32>>,
}

impl State {
    pub fn new(robot_count: usize) -> Self {
        let robots: Vec<_> = (0..robot_count).map(Robot::new).collect();

        Self {
            time_elapsed: Duration::ZERO,
            robots,
            ball: None,
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

            let database = robot.database.clone();
            match database.main_outputs.motion_command {
                MotionCommand::Walk { path, .. } => {
                    if let Some(robot_to_field) =
                        robot.database.main_outputs.robot_to_field.as_mut()
                    {
                        let position = match path[0] {
                            PathSegment::LineSegment(LineSegment(_start, end)) => end,
                            PathSegment::Arc(arc, _orientation) => arc.end,
                        }
                        .coords
                        .cap_magnitude(0.3 * time_step.as_secs_f32());
                        robot_to_field
                            .append_translation_mut(&Translation2::new(position.x, position.y));
                    }
                }
                MotionCommand::InWalkKick { .. } => todo!(),
                _ => {}
            }
        }
        self.time_elapsed += time_step;
    }
}
