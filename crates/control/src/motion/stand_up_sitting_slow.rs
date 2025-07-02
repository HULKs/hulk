use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Robot;
use filtering::low_pass_filter::LowPassFilter;
use framework::deserialize_not_implemented;
use framework::MainOutput;
use hardware::PathsInterface;
use linear_algebra::Vector3;
use motionfile::{InterpolatorState, MotionFile, MotionInterpolator};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
    stand_up::RemainingStandUpDuration,
};

#[derive(Deserialize, Serialize)]
pub struct StandUpSittingSlow {
    #[serde(skip, default = "deserialize_not_implemented")]
    interpolator: MotionInterpolator<Joints<f32>>,
    state: InterpolatorState<Joints<f32>>,
    filtered_gyro: LowPassFilter<nalgebra::Vector3<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
    gyro_low_pass_factor: Parameter<f32, "stand_up_sitting_slow.gyro_low_pass_factor">,
}

#[context]
pub struct CycleContext {
    leg_balancing_factor:
        Parameter<nalgebra::Vector2<f32>, "stand_up_sitting_slow.leg_balancing_factor">,
    speed_factor: Parameter<f32, "stand_up_sitting_slow.speed_factor">,
    condition_input: Input<ConditionInput, "condition_input">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    angular_velocity:
        Input<Vector3<Robot>, "sensor_data.inertial_measurement_unit.angular_velocity">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    stand_up_sitting_slow_estimated_remaining_duration:
        CyclerState<RemainingStandUpDuration, "stand_up_sitting_slow_estimated_remaining_duration">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_sitting_slow_positions: MainOutput<Joints<f32>>,
}

impl StandUpSittingSlow {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("stand_up_sitting.json"))?
                .try_into()?,
            state: InterpolatorState::INITIAL,
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                nalgebra::Vector3::zeros(),
                *context.gyro_low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let estimated_remaining_duration = if context.motion_selection.current_motion
            == MotionType::StandUpSittingSlow
        {
            let last_cycle_duration = context
                .cycle_time
                .last_cycle_duration
                .div_f32(*context.speed_factor);
            let condition_input = context.condition_input;

            self.interpolator
                .advance_state(&mut self.state, last_cycle_duration, condition_input);

            RemainingStandUpDuration::Running(
                self.interpolator
                    .estimated_remaining_duration(self.state)
                    .mul_f32(*context.speed_factor),
            )
        } else {
            self.state.reset();
            RemainingStandUpDuration::NotRunning
        };
        context.motion_safe_exits[MotionType::StandUpSittingSlow] = self.state.is_finished();

        self.filtered_gyro.update(context.angular_velocity.inner);
        let gyro = self.filtered_gyro.state();

        let mut positions = self.interpolator.value(self.state);
        positions.left_leg.ankle_pitch += context.leg_balancing_factor.y * gyro.y;
        positions.left_leg.ankle_roll += context.leg_balancing_factor.x * gyro.x;
        positions.right_leg.ankle_pitch += context.leg_balancing_factor.y * gyro.y;
        positions.right_leg.ankle_roll += context.leg_balancing_factor.x * gyro.x;

        *context.stand_up_sitting_slow_estimated_remaining_duration = estimated_remaining_duration;

        Ok(MainOutputs {
            stand_up_sitting_slow_positions: positions.into(),
        })
    }
}
