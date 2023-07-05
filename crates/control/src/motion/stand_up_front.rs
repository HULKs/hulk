use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use types::{ConditionInput, JointsVelocity};
use types::{
    CycleTime, Joints, MotionCommand, MotionSafeExits, MotionSelection, MotionType, SensorData,
};

pub struct StandUpFront {
    interpolator: MotionInterpolator<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    pub hardware_interface: HardwareInterface,
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
    pub maximum_velocity: Parameter<JointsVelocity, "maximum_joint_velocities">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
    pub should_exit_stand_up_front: PersistentState<bool, "should_exit_stand_up_front">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_front_positions: MainOutput<Joints<f32>>,
    pub stand_up_front_estimated_remaining_duration: MainOutput<Option<Duration>>,
}

impl StandUpFront {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("stand_up_front.json"))?
                .try_into()?,
        })
    }

    pub fn advance_interpolator(&mut self, context: CycleContext) {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        context.motion_safe_exits[MotionType::StandUpFront] = false;

        self.interpolator
            .advance_by(last_cycle_duration, condition_input);

        if self.interpolator.is_finished() {
            context.motion_safe_exits[MotionType::StandUpFront] = true;
        }
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let stand_up_front_estimated_remaining_duration =
            if let MotionType::StandUpFront = context.motion_selection.current_motion {
                self.advance_interpolator(context);
                Some(self.interpolator.estimated_remaining_duration())
            } else {
                self.interpolator.reset();
                None
            };
        Ok(MainOutputs {
            stand_up_front_positions: self.interpolator.value().into(),
            stand_up_front_estimated_remaining_duration:
                stand_up_front_estimated_remaining_duration.into(),
        })
    }
}
