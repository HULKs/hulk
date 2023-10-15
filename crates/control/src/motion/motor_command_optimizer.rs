use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    joints::{arm::ArmJoints, leg::LegJoints, Joints},
    motion_selection::{MotionSelection, MotionType},
    motor_commands::MotorCommand,
    sensor_data::SensorData,
};

#[derive(Default, Deserialize, Serialize)]
pub struct MotorCommandOptimizer {
    position_offset: Joints<f32>,
    state: State,
}

#[derive(Default, Deserialize, Serialize)]
pub enum State {
    #[default]
    Optimizing,
    Resetting,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub motor_commands: Input<MotorCommand<f32>, "motor_commands">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,

    pub offset_reset_threshold: Parameter<f32, "motor_command_optimizer.offset_reset_threshold">,
    pub offset_reset_speed: Parameter<f32, "motor_command_optimizer.offset_reset_speed">,
    pub offset_reset_offset: Parameter<f32, "motor_command_optimizer.offset_reset_offset">,
    pub optimization_speed: Parameter<f32, "motor_command_optimizer.optimization_speed">,
    pub optimization_current_threshold:
        Parameter<f32, "motor_command_optimizer.optimization_current_threshold">,
    pub optimization_sign: Parameter<Joints<f32>, "motor_command_optimizer.optimization_sign">,

    pub squared_position_offset_sum:
        AdditionalOutput<f32, "motor_position_optimization_offset_squared_sum">,
    pub position_offset: AdditionalOutput<Joints<f32>, "motor_position_optimization_offset">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<MotorCommand<f32>>,
}

impl MotorCommandOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let current_motion = context.motion_selection.current_motion;

        let optimization_enabled =
            matches!(current_motion, MotionType::Penalized | MotionType::Stand);

        let currents = context.sensor_data.currents;
        let commands = context.motor_commands;

        let squared_position_offset_sum = self
            .position_offset
            .into_iter()
            .map(|position| position.powf(2.0))
            .sum();

        if squared_position_offset_sum > *context.offset_reset_threshold || !optimization_enabled {
            self.state = State::Resetting;
        }

        match self.state {
            State::Optimizing => {
                let (joint, maximal_current) = currents
                    .enumerate()
                    .max_by(|(_, left), (_, right)| f32::total_cmp(left, right))
                    .unwrap();

                let minimum_not_reached =
                    maximal_current >= *context.optimization_current_threshold;
                if minimum_not_reached {
                    self.position_offset[joint] +=
                        context.optimization_sign[joint] * context.optimization_speed;
                }
            }
            State::Resetting => {
                if current_motion == MotionType::Dispatching {
                    self.position_offset = Joints::default();
                }
                let resetting_finished = squared_position_offset_sum
                    < context.offset_reset_threshold / context.offset_reset_offset;

                if resetting_finished && optimization_enabled {
                    self.state = State::Optimizing;
                } else {
                    self.position_offset = self.position_offset / *context.offset_reset_speed;
                }
            }
        }

        let optimized_knee_pitch_stiffness = if current_motion == MotionType::Penalized {
            0.0
        } else {
            commands.stiffnesses.left_leg.knee_pitch
        };

        let optimized_stiffnesses = Joints {
            left_arm: ArmJoints {
                hand: 0.0,
                ..commands.stiffnesses.left_arm
            },
            right_arm: ArmJoints {
                hand: 0.0,
                ..commands.stiffnesses.right_arm
            },
            left_leg: LegJoints {
                knee_pitch: optimized_knee_pitch_stiffness,
                ..commands.stiffnesses.left_leg
            },
            right_leg: LegJoints {
                knee_pitch: optimized_knee_pitch_stiffness,
                ..commands.stiffnesses.left_leg
            },
            ..commands.stiffnesses
        };

        let optimized_commands = MotorCommand {
            positions: commands.positions + self.position_offset,
            stiffnesses: optimized_stiffnesses,
        };

        context
            .squared_position_offset_sum
            .fill_if_subscribed(|| squared_position_offset_sum);
        context
            .position_offset
            .fill_if_subscribed(|| self.position_offset);

        Ok(MainOutputs {
            optimized_motor_commands: optimized_commands.into(),
        })
    }
}
