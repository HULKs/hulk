use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator};
use nalgebra::Vector2;
use types::{
    ConditionInput, CycleTime, Facing, Joints, MotionCommand, MotionSafeExits, MotionSelection,
    MotionType, SensorData,
};

pub struct StandUpBack {
    interpolator: MotionInterpolator<Joints<f32>>,
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
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "stand_up.gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance: Parameter<f32, "stand_up.gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_back_positions: MainOutput<Joints<f32>>,
}

impl StandUpBack {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/stand_up_back_dortmund_2022.json")?
                .try_into()?,
            filtered_gyro: LowPassFilter::with_roughness_factor(
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

        if context.motion_selection.current_motion == MotionType::StandUpBack {
            self.interpolator
                .advance_by(last_cycle_duration, context.condition_input);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::StandUpBack] = false;
        if self.interpolator.is_finished() {
            match context.motion_command {
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
            stand_up_back_positions: self.interpolator.value().into(),
        })
    }
}
