use std::time::{Duration, SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::point;
use types::{
    foot_bumper_obstacle::FootBumperObstacle, foot_bumper_values::FootBumperValues, CycleTime,
    FallState, SensorData,
};

pub struct FootBumperFilter {
    left_foot_bumper_count: i32,
    right_foot_bumper_count: i32,
    last_left_foot_bumper_time: Option<SystemTime>,
    last_right_foot_bumper_time: Option<SystemTime>,
    left_foot_bumper_pressed_last_cycle: bool,
    right_foot_bumper_pressed_last_cycle: bool,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub foot_bumper_values: AdditionalOutput<FootBumperValues, "foot_bumper_values">,
    pub sensor_angle: Parameter<f32, "foot_bumper_filter.sensor_angle">,
    pub acceptance_duration: Parameter<Duration, "foot_bumper_filter.acceptance_duration">,
    pub activations_needed: Parameter<i32, "foot_bumper_filter.activations_needed">,
    pub obstacle_distance: Parameter<f32, "foot_bumper_filter.obstacle_distance">,

    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub fall_state: Input<FallState, "fall_state">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    // Maybe consider kicks
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub foot_bumper_obstacle: MainOutput<Vec<FootBumperObstacle>>,
}

impl FootBumperFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            left_foot_bumper_count: 0,
            right_foot_bumper_count: 0,
            last_left_foot_bumper_time: None,
            last_right_foot_bumper_time: None,
            left_foot_bumper_pressed_last_cycle: false,
            right_foot_bumper_pressed_last_cycle: false,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let foot_bumper_sensors = &context.sensor_data.force_sensitive_resistors;
        let fall_state = context.fall_state;

        if foot_bumper_sensors.left.sum() > 1.0 {
            if !self.left_foot_bumper_pressed_last_cycle {
                self.left_foot_bumper_count += 1;
                self.left_foot_bumper_pressed_last_cycle = true;
                self.last_left_foot_bumper_time = Some(SystemTime::now())
            }
        } else {
            self.left_foot_bumper_pressed_last_cycle = false;
        }

        if foot_bumper_sensors.right.sum() > 1.0 {
            if !self.right_foot_bumper_pressed_last_cycle {
                self.right_foot_bumper_count += 1;
                self.right_foot_bumper_pressed_last_cycle = true;
                self.last_right_foot_bumper_time = Some(SystemTime::now())
            }
        } else {
            self.right_foot_bumper_pressed_last_cycle = false;
        }

        if let Some(last_left_foot_bumper_time) = self.last_left_foot_bumper_time {
            match last_left_foot_bumper_time.elapsed() {
                Ok(last_left_foot_bumper_duration) => {
                    if last_left_foot_bumper_duration > *context.acceptance_duration {
                        self.last_left_foot_bumper_time = None;
                        self.left_foot_bumper_count = 0;
                        self.left_foot_bumper_pressed_last_cycle = false;
                    }
                }
                Err(e) => {
                    eprintln!("Duration elapsed failed: {e:?}");
                    self.last_left_foot_bumper_time = None;
                    self.left_foot_bumper_count = 0;
                    self.left_foot_bumper_pressed_last_cycle = false;
                }
            }
        }
        if let Some(last_right_foot_bumper_time) = self.last_right_foot_bumper_time {
            match last_right_foot_bumper_time.elapsed() {
                Ok(last_right_foot_bumper_duration) => {
                    if last_right_foot_bumper_duration > *context.acceptance_duration {
                        self.last_right_foot_bumper_time = None;
                        self.right_foot_bumper_count = 0;
                        self.right_foot_bumper_pressed_last_cycle = false;
                    }
                }
                Err(e) => {
                    eprintln!("Duration elapsed failed: {e:?}");
                    self.last_right_foot_bumper_time = None;
                    self.right_foot_bumper_count = 0;
                    self.right_foot_bumper_pressed_last_cycle = false;
                }
            }
        }

        let obstacle_detected_on_left = self.left_foot_bumper_count >= *context.activations_needed;
        let obstacle_detected_on_right =
            self.right_foot_bumper_count >= *context.activations_needed;

        let left_point = point![
            context.sensor_angle.cos() * *context.obstacle_distance,
            context.sensor_angle.sin() * *context.obstacle_distance
        ];
        let right_point = point![
            context.sensor_angle.cos() * *context.obstacle_distance,
            -context.sensor_angle.sin() * *context.obstacle_distance
        ];
        let middle_point = point![*context.obstacle_distance, 0.0];

        let obstacle_positions = match (
            fall_state,
            obstacle_detected_on_left,
            obstacle_detected_on_right,
        ) {
            (FallState::Upright, true, true) => vec![middle_point],
            (FallState::Upright, true, false) => vec![left_point],
            (FallState::Upright, false, true) => vec![right_point],
            _ => vec![],
        };
        let foot_bumper_obstacles: Vec<_> = obstacle_positions
            .iter()
            .map(|position_in_robot| FootBumperObstacle {
                position_in_robot: *position_in_robot,
            })
            .collect();

        context
            .foot_bumper_values
            .fill_if_subscribed(|| FootBumperValues {
                left_foot_bumper_count: self.left_foot_bumper_count,
                right_foot_bumper_count: self.right_foot_bumper_count,
                obstacle_deteced_on_left: obstacle_detected_on_left,
                obstacle_deteced_on_right: obstacle_detected_on_right,
            });

        Ok(MainOutputs {
            foot_bumper_obstacle: foot_bumper_obstacles.into(),
        })
    }
}
