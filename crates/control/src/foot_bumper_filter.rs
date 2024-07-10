use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use color_eyre::Result;
use coordinate_systems::Ground;
use linear_algebra::{point, Rotation2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};

use types::{
    cycle_time::CycleTime, fall_state::FallState, foot_bumper_obstacle::FootBumperObstacle,
    foot_bumper_values::FootBumperValues, sensor_data::SensorData,
};

#[derive(Default, Deserialize, Serialize)]
pub struct FootBumperFilter {
    left_in_use: bool,
    right_in_use: bool,
    left_detection_buffer: VecDeque<bool>,
    right_detection_buffer: VecDeque<bool>,
    left_count: i32,
    right_count: i32,
    last_left_time: Option<SystemTime>,
    last_right_time: Option<SystemTime>,
    left_pressed_last_cycle: bool,
    right_pressed_last_cycle: bool,
}

#[context]
pub struct CreationContext {
    buffer_size: Parameter<usize, "foot_bumper_filter.buffer_size">,
}

#[context]
pub struct CycleContext {
    acceptance_duration: Parameter<Duration, "foot_bumper_filter.acceptance_duration">,
    activations_needed: Parameter<i32, "foot_bumper_filter.activations_needed">,
    enabled: Parameter<bool, "obstacle_filter.use_foot_bumper_measurements">,
    number_of_detections_in_buffer_for_defective_declaration: Parameter<
        usize,
        "foot_bumper_filter.number_of_detections_in_buffer_for_defective_declaration",
    >,
    number_of_detections_in_buffer_to_reset_in_use:
        Parameter<usize, "foot_bumper_filter.number_of_detections_in_buffer_to_reset_in_use">,
    obstacle_distance: Parameter<f32, "foot_bumper_filter.obstacle_distance">,
    sensor_angle: Parameter<f32, "foot_bumper_filter.sensor_angle">,

    fall_state: Input<FallState, "fall_state">,
    sensor_data: Input<SensorData, "sensor_data">,
    cycle_time: Input<CycleTime, "cycle_time">,

    foot_bumper_values: AdditionalOutput<FootBumperValues, "foot_bumper_values">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub foot_bumper_obstacle: MainOutput<Vec<FootBumperObstacle>>,
}

impl FootBumperFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            left_in_use: true,
            right_in_use: true,
            left_detection_buffer: VecDeque::from(vec![false; *context.buffer_size]),
            right_detection_buffer: VecDeque::from(vec![false; *context.buffer_size]),
            ..Default::default()
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let fall_state = context.fall_state;

        if !context.enabled {
            return Ok(MainOutputs::default());
        }

        let touch_sensors = &context.sensor_data.touch_sensors;
        if touch_sensors.left_foot_left || touch_sensors.left_foot_right {
            if !self.left_pressed_last_cycle {
                self.left_count += 1;
                self.left_pressed_last_cycle = true;
                self.last_left_time = Some(context.cycle_time.start_time);
            }
        } else {
            self.left_pressed_last_cycle = false;
        }

        if touch_sensors.right_foot_left || touch_sensors.right_foot_right {
            if !self.right_pressed_last_cycle {
                self.right_count += 1;
                self.right_pressed_last_cycle = true;
                self.last_right_time = Some(context.cycle_time.start_time);
            }
        } else {
            self.right_pressed_last_cycle = false;
        }

        if let Some(last_left_foot_bumper_time) = self.last_left_time {
            if last_left_foot_bumper_time
                .elapsed()
                .expect("Time ran backwards")
                > *context.acceptance_duration
            {
                self.last_left_time = None;
                self.left_count = 0;
                self.left_pressed_last_cycle = false;
            }
        }

        if let Some(last_right_foot_bumper_time) = self.last_right_time {
            if last_right_foot_bumper_time
                .elapsed()
                .expect("Time ran backwards")
                > *context.acceptance_duration
            {
                self.last_right_time = None;
                self.right_count = 0;
                self.right_pressed_last_cycle = false;
            }
        }
        self.left_detection_buffer
            .push_back(self.left_pressed_last_cycle);
        self.left_detection_buffer.pop_front();
        self.right_detection_buffer
            .push_back(self.right_pressed_last_cycle);
        self.right_detection_buffer.pop_front();

        let obstacle_detected_on_left = self.left_count >= *context.activations_needed;
        let obstacle_detected_on_right = self.right_count >= *context.activations_needed;

        self.check_for_bumper_errors(&context);

        if *fall_state != FallState::Upright {
            return Ok(Default::default());
        }

        let obstacle_angle = match (
            obstacle_detected_on_left && self.left_in_use,
            obstacle_detected_on_right && self.right_in_use,
        ) {
            (true, true) => 0.0,
            (true, false) => *context.sensor_angle,
            (false, true) => -context.sensor_angle,
            _ => return Ok(Default::default()),
        };
        let obstacle_position = Rotation2::<Ground, Ground>::new(obstacle_angle)
            * point![*context.obstacle_distance, 0.0];

        context
            .foot_bumper_values
            .fill_if_subscribed(|| FootBumperValues {
                left_foot_bumper_count: self.left_count,
                right_foot_bumper_count: self.right_count,
                obstacle_detected_on_left,
                obstacle_detected_on_right,
            });

        Ok(MainOutputs {
            foot_bumper_obstacle: vec![obstacle_position.into()].into(),
        })
    }

    fn check_for_bumper_errors(&mut self, context: &CycleContext) {
        let left_count: usize = self.left_detection_buffer.iter().filter(|x| **x).count();

        if left_count >= *context.number_of_detections_in_buffer_for_defective_declaration {
            self.left_in_use = false;
        }
        let right_count: usize = self.right_detection_buffer.iter().filter(|x| **x).count();

        if right_count >= *context.number_of_detections_in_buffer_for_defective_declaration {
            self.right_in_use = false;
        }
        if !self.left_in_use
            && left_count <= *context.number_of_detections_in_buffer_to_reset_in_use
        {
            self.left_in_use = true;
        }
        if !self.right_in_use
            && right_count <= *context.number_of_detections_in_buffer_to_reset_in_use
        {
            self.right_in_use = true;
        }
    }
}
