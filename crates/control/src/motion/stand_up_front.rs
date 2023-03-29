use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use nalgebra::Vector2;
use types::{
    CycleTime, Facing, Joints, MotionCommand, MotionFile, MotionSafeExits, MotionSelection,
    MotionType, SensorData,
};

use crate::spline_interpolator::SplineInterpolator;

pub struct StandUpFront {
    interpolator: SplineInterpolator,
    filtered_gyro: LowPassFilter<Vector2<f32>>,
}

#[context]
pub struct CreationContext {
    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "stand_up.gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance: Parameter<f32, "stand_up.gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "stand_up.gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance: Parameter<f32, "stand_up.gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_front_positions: MainOutput<Joints>,
}

impl StandUpFront {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/stand_up_front.json")?.try_into()?,
            filtered_gyro: LowPassFilter::with_alpha(
                Vector2::zeros(),
                *context.gyro_low_pass_filter_coefficient,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let angular_velocity = context
            .sensor_data
            .inertial_measurement_unit
            .angular_velocity;

        self.filtered_gyro
            .update(Vector2::new(angular_velocity.x, angular_velocity.y));

        if context.motion_selection.current_motion == MotionType::StandUpFront {
            self.interpolator.advance_by(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::StandUpFront] = false;
        if self.interpolator.is_finished() {
            match context.motion_command {
                MotionCommand::StandUp {
                    facing: Facing::Down,
                } => self.interpolator.reset(),
                _ => {
                    if self.filtered_gyro.state().abs()
                        < Vector2::new(
                            *context.gyro_low_pass_filter_tolerance,
                            *context.gyro_low_pass_filter_tolerance,
                        )
                    {
                        context.motion_safe_exits[MotionType::StandUpFront] = true;
                    }
                }
            };
        }

        Ok(MainOutputs {
            stand_up_front_positions: self
                .interpolator
                .value()
                .wrap_err("error computing interpolation in stand up front")?
                .into(),
        })
    }
}
