use std::time::Duration;

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
use types::fall_state::StandUpSpeed;
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
    stand_up::RemainingStandUpDuration,
};

#[derive(Deserialize, Serialize)]
pub struct StandUpFront {
    #[serde(skip, default = "deserialize_not_implemented")]
    interpolator: MotionInterpolator<Joints<f32>>,
    state: InterpolatorState<Joints<f32>>,
    slow_state: InterpolatorState<Joints<f32>>,
    filtered_gyro: LowPassFilter<nalgebra::Vector3<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
    gyro_low_pass_factor: Parameter<f32, "stand_up_front.gyro_low_pass_factor">,
}

#[context]
pub struct CycleContext {
    leg_balancing_factor: Parameter<nalgebra::Vector2<f32>, "stand_up_front.leg_balancing_factor">,
    speed_factor: Parameter<f32, "stand_up_front.slow_speed_factor">,

    condition_input: Input<ConditionInput, "condition_input">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    angular_velocity:
        Input<Vector3<Robot>, "sensor_data.inertial_measurement_unit.angular_velocity">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    stand_up_front_estimated_remaining_duration:
        CyclerState<RemainingStandUpDuration, "stand_up_front_estimated_remaining_duration">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_front_positions: MainOutput<Joints<f32>>,
}

impl StandUpFront {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        Ok(Self {
            interpolator: MotionFile::from_path(paths.motions.join("stand_up_front.json"))?
                .try_into()?,
            state: InterpolatorState::INITIAL,
            slow_state: InterpolatorState::INITIAL,
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                nalgebra::Vector3::zeros(),
                *context.gyro_low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        let (mut positions, estimated_remaining_duration) = match &context
            .motion_selection
            .current_motion
        {
            MotionType::StandUpFront(speed) => match speed {
                StandUpSpeed::Default => {
                    self.slow_state.reset();

                    self.interpolator.advance_state(
                        &mut self.state,
                        last_cycle_duration,
                        condition_input,
                    );

                    (
                        self.interpolator.value(self.state),
                        RemainingStandUpDuration::Running(
                            self.interpolator
                                .estimated_remaining_duration(self.state)
                                .unwrap_or(Duration::MAX),
                        ),
                    )
                }
                StandUpSpeed::Slow => {
                    self.state.reset();

                    self.interpolator.advance_state(
                        &mut self.slow_state,
                        last_cycle_duration.mul_f32(*context.speed_factor),
                        condition_input,
                    );

                    let corrected_estimated_remaining_duration = self
                        .interpolator
                        .estimated_remaining_duration(self.slow_state)
                        .map(|duration| duration.div_f32(*context.speed_factor))
                        .unwrap_or(Duration::MAX);

                    (
                        self.interpolator.value(self.slow_state),
                        RemainingStandUpDuration::Running(corrected_estimated_remaining_duration),
                    )
                }
            },
            _ => {
                self.state.reset();
                self.slow_state.reset();

                (
                    self.interpolator.value(self.state),
                    RemainingStandUpDuration::NotRunning,
                )
            }
        };
        context.motion_safe_exits[MotionType::StandUpFront(StandUpSpeed::Default)] =
            self.state.is_finished();
        context.motion_safe_exits[MotionType::StandUpFront(StandUpSpeed::Slow)] =
            self.slow_state.is_finished();

        self.filtered_gyro.update(context.angular_velocity.inner);
        let gyro = self.filtered_gyro.state();

        positions.left_leg.ankle_pitch += context.leg_balancing_factor.y * gyro.y;
        positions.left_leg.ankle_roll += context.leg_balancing_factor.x * gyro.x;
        positions.left_leg.hip_yaw_pitch += context.leg_balancing_factor.x * gyro.x;
        positions.right_leg.ankle_pitch += context.leg_balancing_factor.y * gyro.y;
        positions.right_leg.ankle_roll += context.leg_balancing_factor.x * gyro.x;
        positions.right_leg.hip_yaw_pitch += context.leg_balancing_factor.x * gyro.x;

        *context.stand_up_front_estimated_remaining_duration = estimated_remaining_duration;

        Ok(MainOutputs {
            stand_up_front_positions: positions.into(),
        })
    }
}
