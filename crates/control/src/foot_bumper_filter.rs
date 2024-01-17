use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::point;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use types::{
    cycle_time::CycleTime, fall_state::FallState, foot_bumper_obstacle::FootBumperObstacle,
    foot_bumper_values::FootBumperValues, sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct FootBumperFilter {
    left_count: i32,
    right_count: i32,
    last_left_time: Option<SystemTime>,
    last_right_time: Option<SystemTime>,
    left_pressed_last_cycle: bool,
    right_pressed_last_cycle: bool,
    left_in_use: bool,
    right_in_use: bool,
    left_detection_buffer: VecDeque<bool>,
    right_detection_buffer: VecDeque<bool>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub foot_bumper_values: AdditionalOutput<FootBumperValues, "foot_bumper_values">,
    pub acceptance_duration: Parameter<Duration, "foot_bumper_filter.acceptance_duration">,
    pub activations_needed: Parameter<i32, "foot_bumper_filter.activations_needed">,
    pub buffer_size: Parameter<i32, "foot_bumper_filter.buffer_size">,
    pub enabled: Parameter<bool, "foot_bumper_filter.enabled">,
    pub number_of_true_elements_in_buffer_for_defective_declaration: Parameter<
        i32,
        "foot_bumper_filter.number_of_true_elements_in_buffer_for_defective_declaration",
    >,
    pub number_of_true_elements_in_buffer_to_reset_in_use:
        Parameter<i32, "foot_bumper_filter.number_of_true_elements_in_buffer_to_reset_in_use">,
    pub obstacle_distance: Parameter<f32, "foot_bumper_filter.obstacle_distance">,
    pub sensor_angle: Parameter<f32, "foot_bumper_filter.sensor_angle">,

    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub fall_state: Input<FallState, "fall_state">,
    pub sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub foot_bumper_obstacle: MainOutput<Vec<FootBumperObstacle>>,
}

impl FootBumperFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            left_count: 0,
            right_count: 0,
            last_left_time: None,
            last_right_time: None,
            left_pressed_last_cycle: false,
            right_pressed_last_cycle: false,
            left_in_use: true,
            right_in_use: true,
            left_detection_buffer: VecDeque::with_capacity(15),
            right_detection_buffer: VecDeque::with_capacity(15),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let fall_state = context.fall_state;

        if !context.enabled {
            return Ok(MainOutputs::default());
        }

        if context.sensor_data.touch_sensors.left_foot_left
            || context.sensor_data.touch_sensors.left_foot_right
        {
            if !self.left_pressed_last_cycle {
                self.left_count += 1;
                self.left_pressed_last_cycle = true;
                self.last_left_time = Some(SystemTime::now());
            }
        } else {
            self.left_pressed_last_cycle = false;
        }

        if context.sensor_data.touch_sensors.right_foot_left
            || context.sensor_data.touch_sensors.right_foot_right
        {
            if !self.right_pressed_last_cycle {
                self.right_count += 1;
                self.right_pressed_last_cycle = true;
                self.last_right_time = Some(SystemTime::now());
            }
        } else {
            self.right_pressed_last_cycle = false;
        }

        if let Some(last_left_foot_bumper_time) = self.last_left_time {
            match last_left_foot_bumper_time.elapsed() {
                Ok(last_left_foot_bumper_duration) => {
                    if last_left_foot_bumper_duration > *context.acceptance_duration {
                        self.last_left_time = None;
                        self.left_count = 0;
                        self.left_pressed_last_cycle = false;
                    }
                }
                Err(e) => {
                    eprintln!("Duration elapsed failed: {e:?}");
                    self.last_left_time = None;
                    self.left_count = 0;
                    self.left_pressed_last_cycle = false;
                }
            }
        }
        if let Some(last_right_foot_bumper_time) = self.last_right_time {
            match last_right_foot_bumper_time.elapsed() {
                Ok(last_right_foot_bumper_duration) => {
                    if last_right_foot_bumper_duration > *context.acceptance_duration {
                        self.last_right_time = None;
                        self.right_count = 0;
                        self.right_pressed_last_cycle = false;
                    }
                }
                Err(e) => {
                    eprintln!("Duration elapsed failed: {e:?}");
                    self.last_right_time = None;
                    self.right_count = 0;
                    self.right_pressed_last_cycle = false;
                }
            }
        }

        let obstacle_detected_on_left = self.left_count >= *context.activations_needed;
        let obstacle_detected_on_right = self.right_count >= *context.activations_needed;
        self.left_detection_buffer
            .push_back(obstacle_detected_on_left);
        self.right_detection_buffer
            .push_back(obstacle_detected_on_right);

        let left_count: i32 = self
            .left_detection_buffer
            .iter()
            .filter(|x| **x)
            .count()
            .try_into()
            .unwrap();
        if left_count >= *context.number_of_true_elements_in_buffer_for_defective_declaration {
            self.left_in_use = false;
        }
        let right_count: i32 = self
            .left_detection_buffer
            .iter()
            .filter(|x| **x)
            .count()
            .try_into()
            .unwrap();
        if right_count >= *context.number_of_true_elements_in_buffer_for_defective_declaration {
            self.right_in_use = false;
        }
        if !self.left_in_use
            && left_count <= *context.number_of_true_elements_in_buffer_to_reset_in_use
        {
            self.left_in_use = true;
        }
        if !self.right_in_use
            && right_count <= *context.number_of_true_elements_in_buffer_to_reset_in_use
        {
            self.right_in_use = true;
        }

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
            self.left_in_use,
            self.right_in_use,
        ) {
            (FallState::Upright, true, true, true, true) => vec![middle_point],
            (FallState::Upright, true, false, true, _) => vec![left_point],
            (FallState::Upright, false, true, _, true) => vec![right_point],
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
                left_foot_bumper_count: self.left_count,
                right_foot_bumper_count: self.right_count,
                obstacle_deteced_on_left: obstacle_detected_on_left,
                obstacle_deteced_on_right: obstacle_detected_on_right,
            });

        Ok(MainOutputs {
            foot_bumper_obstacle: foot_bumper_obstacles.into(),
        })
    }
}
