use module_derive::{module, require_some};
use nalgebra::Vector2;
use types::{
    Facing, Joints, MotionCommand, MotionSafeExits, MotionSelection, MotionType, SensorData,
};

use crate::control::filtering::LowPassFilter;

use super::motion_file::{MotionFile, MotionFileInterpolator};

pub struct StandUpBack {
    interpolator: MotionFileInterpolator,
    filtered_gyro: LowPassFilter<Vector2<f32>>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = motion_command, data_type = MotionCommand)]
#[parameter(path = control.stand_up.gyro_low_pass_filter_coefficient, data_type = f32)]
#[parameter(path = control.stand_up.gyro_low_pass_filter_tolerance, data_type = f32)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[main_output(name = stand_up_back_positions, data_type = Joints)]
impl StandUpBack {}

impl StandUpBack {
    fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/stand_up_back.json")?.into(),
            filtered_gyro: LowPassFilter::with_alpha(
                Vector2::zeros(),
                *context.gyro_low_pass_filter_coefficient,
            ),
        })
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let last_cycle_duration = require_some!(context.sensor_data)
            .cycle_info
            .last_cycle_duration;
        let angular_velocity = require_some!(context.sensor_data)
            .inertial_measurement_unit
            .angular_velocity;
        let motion_selection = require_some!(context.motion_selection);
        let motion_command = require_some!(context.motion_command);

        self.filtered_gyro
            .update(Vector2::new(angular_velocity.x, angular_velocity.y));

        if motion_selection.current_motion == MotionType::StandUpBack {
            self.interpolator.step(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::StandUpBack] = false;
        if self.interpolator.is_finished() {
            match motion_command {
                MotionCommand::StandUp { facing: Facing::Up } => self.interpolator.reset(),
                _ => {
                    if self.filtered_gyro.state().abs()
                        < Vector2::new(
                            *context.gyro_low_pass_filter_tolerance,
                            *context.gyro_low_pass_filter_tolerance,
                        )
                    {
                        context.motion_safe_exits[MotionType::StandUpBack] = true;
                    }
                }
            };
        }

        Ok(MainOutputs {
            stand_up_back_positions: Some(self.interpolator.value()),
        })
    }
}
