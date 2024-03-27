use color_eyre::Result;
use context_attribute::context;
use energy_optimization::{current_minimizer::CurrentMinimizer, CurrentMinimizerParameters};
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{body::BodyJoints, head::HeadJoints, Joints},
    motion_file_player::MotionFileState,
    motion_selection::{MotionSelection, MotionVariant},
    motor_commands::MotorCommands,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandCollector {
    current_minimizer: CurrentMinimizer,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    arms_up_squat: Input<MotionFileState, "arms_up_squat">,
    dispatching_command: Input<MotorCommands<Joints<f32>>, "dispatching_command">,
    fall_protection: Input<MotorCommands<Joints<f32>>, "fall_protection_command">,
    head_joints: Input<MotorCommands<HeadJoints<f32>>, "head_joints_command">,
    jump_left: Input<MotionFileState, "jump_left">,
    jump_right: Input<MotionFileState, "jump_right">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    sensor_data: Input<SensorData, "sensor_data">,
    sit_down: Input<MotionFileState, "sit_down">,
    stand_up_back: Input<MotionFileState, "stand_up_back">,
    stand_up_front: Input<MotionFileState, "stand_up_front">,
    stand_up_sitting: Input<MotionFileState, "stand_up_sitting">,
    stand_up_squatting: Input<MotionFileState, "stand_up_squatting">,
    walk: Input<MotorCommands<BodyJoints<f32>>, "walk_motor_commands">,
    cycle_time: Input<CycleTime, "cycle_time">,

    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    initial_pose: Parameter<Joints<f32>, "initial_pose">,
    current_minimizer_parameters:
        Parameter<CurrentMinimizerParameters, "current_minimizer_parameters">,

    motor_position_difference: AdditionalOutput<Joints<f32>, "motor_positions_difference">,
    current_minimizer: AdditionalOutput<CurrentMinimizer, "current_minimizer">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motor_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl MotorCommandCollector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_minimizer: CurrentMinimizer::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let measured_positions = context.sensor_data.positions;

        let motor_commands = match context.motion_selection.current_motion {
            MotionVariant::ArmsUpSquat => context.arms_up_squat.commands,
            MotionVariant::Dispatching => {
                self.current_minimizer.reset();
                *context.dispatching_command
            }
            MotionVariant::FallProtection => *context.fall_protection,
            MotionVariant::Initial => MotorCommands {
                positions: self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    *context.initial_pose,
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                stiffnesses: Joints::fill(0.6),
            },
            MotionVariant::JumpLeft => context.jump_left.commands,
            MotionVariant::JumpRight => context.jump_right.commands,
            MotionVariant::Penalized => MotorCommands {
                positions: self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    *context.penalized_pose,
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                stiffnesses: Joints::fill(0.6),
            },
            MotionVariant::SitDown => context.sit_down.commands,
            MotionVariant::Stand => MotorCommands {
                positions: self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    Joints::from_head_and_body(
                        context.head_joints.positions,
                        context.walk.positions,
                    ),
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                stiffnesses: Joints::from_head_and_body(
                    context.head_joints.stiffnesses,
                    context.walk.stiffnesses,
                ),
            },
            MotionVariant::StandUpBack => context.stand_up_back.commands,
            MotionVariant::StandUpFront => context.stand_up_front.commands,
            MotionVariant::StandUpSitting => context.stand_up_sitting.commands,
            MotionVariant::StandUpSquatting => context.stand_up_squatting.commands,
            MotionVariant::Unstiff => MotorCommands {
                positions: measured_positions,
                stiffnesses: Joints::fill(0.0),
            },
            MotionVariant::Walk => MotorCommands {
                positions: Joints::from_head_and_body(
                    context.head_joints.positions,
                    context.walk.positions,
                ),
                stiffnesses: Joints::from_head_and_body(
                    context.head_joints.stiffnesses,
                    context.walk.stiffnesses,
                ),
            },
        };

        // The actuators use the raw sensor data (not corrected like current_positions) in their feedback loops,
        // thus the compensation is required to make them reach the actual desired position.
        let compensated_positions = motor_commands.positions + *context.joint_calibration_offsets;
        let motor_commands = MotorCommands {
            positions: compensated_positions,
            stiffnesses: motor_commands.stiffnesses,
        };

        context
            .motor_position_difference
            .fill_if_subscribed(|| motor_commands.positions - measured_positions);

        context
            .current_minimizer
            .fill_if_subscribed(|| self.current_minimizer);

        Ok(MainOutputs {
            motor_commands: motor_commands.into(),
        })
    }
}
