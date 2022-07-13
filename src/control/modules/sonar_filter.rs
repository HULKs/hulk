use crate::control::filtering::LowPassFilter;
use module_derive::module;
use nalgebra::point;
use types::{FallState, SensorData, SonarObstacle, SonarValues};

pub struct SonarFilter {
    filtered_sonar_left: LowPassFilter<f32>,
    filtered_sonar_right: LowPassFilter<f32>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData, required)]
#[input(path = fall_state, data_type = FallState, required)]
#[parameter(path = control.sonar_filter.low_pass_filter_coefficient, data_type = f32)]
#[parameter(path = control.sonar_filter.maximal_reliable_distance, data_type = f32)]
#[parameter(path = control.sonar_filter.minimal_reliable_distance, data_type = f32)]
#[parameter(path = control.sonar_filter.maximal_detectable_distance, data_type = f32)]
#[parameter(path = control.sonar_obstacle.sensor_angle, data_type = f32)]
#[additional_output(path = sonar_values, data_type = SonarValues)]
#[main_output(data_type = SonarObstacle)]
impl SonarFilter {}

impl SonarFilter {
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            filtered_sonar_left: LowPassFilter::with_alpha(
                *context.maximal_detectable_distance,
                *context.low_pass_filter_coefficient,
            ),
            filtered_sonar_right: LowPassFilter::with_alpha(
                *context.maximal_detectable_distance,
                *context.low_pass_filter_coefficient,
            ),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sonar_sensors = &context.sensor_data.sonar_sensors;
        let fall_state = context.fall_state;

        self.filtered_sonar_left.update(sonar_sensors.left);
        self.filtered_sonar_right.update(sonar_sensors.right);

        let acceptance_range =
            *context.minimal_reliable_distance..*context.maximal_reliable_distance;

        let obstacle_detected_on_left =
            (acceptance_range).contains(&self.filtered_sonar_left.state());
        let obstacle_detected_on_right =
            (acceptance_range).contains(&self.filtered_sonar_right.state());

        context.sonar_values.fill_on_subscription(|| SonarValues {
            left_sonar: obstacle_detected_on_left,
            right_sonar: obstacle_detected_on_right,
            filtered_left_sonar_value: self.filtered_sonar_left.state(),
            filtered_right_sonar_value: self.filtered_sonar_right.state(),
        });

        let left_point = point![
            context.sensor_angle.cos() * self.filtered_sonar_left.state(),
            context.sensor_angle.sin() * self.filtered_sonar_left.state()
        ];
        let right_point = point![
            context.sensor_angle.cos() * self.filtered_sonar_right.state(),
            -context.sensor_angle.sin() * self.filtered_sonar_right.state()
        ];

        let obstacle_position = match (
            fall_state,
            obstacle_detected_on_left,
            obstacle_detected_on_right,
        ) {
            (FallState::Upright, true, true) => {
                if self.filtered_sonar_left.state() < self.filtered_sonar_right.state() {
                    Some(left_point)
                } else {
                    Some(right_point)
                }
            }
            (FallState::Upright, true, false) => Some(left_point),
            (FallState::Upright, false, true) => Some(right_point),
            _ => None,
        };
        let sonar_obstacle =
            obstacle_position.map(|position_in_robot| SonarObstacle { position_in_robot });

        Ok(MainOutputs { sonar_obstacle })
    }
}
