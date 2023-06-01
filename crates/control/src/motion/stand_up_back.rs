use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator};
use types::{ConditionInput, JointsVelocity};
use types::{
    CycleTime, Joints, MotionCommand, MotionSafeExits, MotionSelection, MotionType, SensorData,
};

pub struct StandUpBack {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {}

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
    pub maximum_velocity: Parameter<JointsVelocity, "maximum_joint_velocities">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_back_positions: MainOutput<Joints<f32>>,
}

impl StandUpBack {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/stand_up_back_dortmund_2022.json")?
                .try_into()?,
        })
    }

    pub fn advance_interpolator(&mut self, context: CycleContext) {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        context.motion_safe_exits[MotionType::StandUpBack] = false;

        self.interpolator
            .advance_by(last_cycle_duration, condition_input);

        if self.interpolator.is_finished() {
            context.motion_safe_exits[MotionType::StandUpBack] = true;
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if let MotionType::StandUpBack = context.motion_selection.current_motion {
            self.advance_interpolator(context);
        } else {
            self.interpolator.reset();
        };
        Ok(MainOutputs {
            stand_up_back_positions: self.interpolator.value().into(),
        })
    }
}
