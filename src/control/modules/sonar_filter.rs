use crate::control::filtering::LowPassFilter;
use module_derive::{module, require_some};
use nalgebra::{point, Point2};
use types::{SensorData, SonarObstacle, SonarValues};

pub struct SonarFilter {
    filtered_sonar_left: LowPassFilter<f32>,
    filtered_sonar_right: LowPassFilter<f32>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[parameter(path = control.sonar_filter.low_pass_filter_coefficient, data_type = f32)]
#[parameter(path = control.sonar_filter.distance_threshold, data_type = f32)]
#[parameter(path = control.sonar_obstacle.single_offset, data_type = Point2<f32>)]
#[parameter(path = control.sonar_obstacle.double_offset, data_type = Point2<f32>)]
#[additional_output(path = sonar_values, data_type = SonarValues)]
#[main_output(data_type = SonarObstacle)]
impl SonarFilter {}

impl SonarFilter {
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            filtered_sonar_left: LowPassFilter::with_alpha(
                0.0,
                *context.low_pass_filter_coefficient,
            ),
            filtered_sonar_right: LowPassFilter::with_alpha(
                0.0,
                *context.low_pass_filter_coefficient,
            ),
        })
    }

    fn cycle(&mut self, mut context: CycleContext) -> anyhow::Result<MainOutputs> {
        let sonar_sensors = &require_some!(context.sensor_data).sonar_sensors;
        self.filtered_sonar_left.update(sonar_sensors.left);
        self.filtered_sonar_right.update(sonar_sensors.right);

        let left = self.filtered_sonar_left.state() < *context.distance_threshold;
        let right = self.filtered_sonar_right.state() < *context.distance_threshold;

        context.sonar_values.fill_on_subscription(|| SonarValues {
            left_sonar: left,
            right_sonar: right,
            filtered_left_sonar_value: self.filtered_sonar_left.state(),
            filtered_right_sonar_value: self.filtered_sonar_right.state(),
        });

        Ok(MainOutputs {
            sonar_obstacle: match (left, right) {
                (true, true) => Some(SonarObstacle {
                    offset: *context.double_offset,
                }),
                (true, false) => Some(SonarObstacle {
                    offset: *context.single_offset,
                }),
                (false, true) => Some(SonarObstacle {
                    offset: point![(*context.single_offset).x, -(*context.single_offset).y],
                }),
                (false, false) => None,
            },
        })
    }
}
