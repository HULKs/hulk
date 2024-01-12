use color_eyre::Result;
use context_attribute::context;
use energy_optimization::current_minimizer::CurrentMinimizer;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    joints::{body::BodyJoints, head::HeadJoints, Joints},
    motion_selection::{MotionSelection, MotionType},
    motor_commands::MotorCommands,
    parameters::CurrentMinimizerParameters,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandCollector {
    current_minimizer: CurrentMinimizer,
}

#[context]
pub struct CreationContext {
    current_minimizer_parameters:
        Parameter<CurrentMinimizerParameters, "current_minimizer_parameters">,
}

#[context]
pub struct CycleContext {
    arms_up_squat_joints_command: Input<MotorCommands<Joints<f32>>, "arms_up_squat_joints_command">,
    dispatching_command: Input<MotorCommands<Joints<f32>>, "dispatching_command">,
    fall_protection_command: Input<MotorCommands<Joints<f32>>, "fall_protection_command">,
    head_joints_command: Input<MotorCommands<HeadJoints<f32>>, "head_joints_command">,
    jump_left_joints_command: Input<MotorCommands<Joints<f32>>, "jump_left_joints_command">,
    jump_right_joints_command: Input<MotorCommands<Joints<f32>>, "jump_right_joints_command">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    sensor_data: Input<SensorData, "sensor_data">,
    sit_down_joints_command: Input<MotorCommands<Joints<f32>>, "sit_down_joints_command">,
    stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    walk_motor_commands: Input<MotorCommands<BodyJoints<f32>>, "walk_motor_commands">,

    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    initial_pose: Parameter<Joints<f32>, "initial_pose">,

    motor_position_difference: AdditionalOutput<Joints<f32>, "motor_positions_difference">,
    penalized_current_minimizer: AdditionalOutput<CurrentMinimizer, "penalized_current_minimizer">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motor_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl MotorCommandCollector {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_minimizer: CurrentMinimizer {
                parameters: *context.current_minimizer_parameters,
                ..Default::default()
            },
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let current_positions = context.sensor_data.positions;
        let dispatching_command = context.dispatching_command;
        let fall_protection_positions = context.fall_protection_command.positions;
        let fall_protection_stiffnesses = context.fall_protection_command.stiffnesses;
        let head_joints_command = context.head_joints_command;
        let motion_selection = context.motion_selection;
        let arms_up_squat = context.arms_up_squat_joints_command;
        let jump_left = context.jump_left_joints_command;
        let jump_right = context.jump_right_joints_command;
        let sit_down = context.sit_down_joints_command;
        let stand_up_back_positions = context.stand_up_back_positions;
        let stand_up_front_positions = context.stand_up_front_positions;
        let walk = context.walk_motor_commands;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::Dispatching => (
                dispatching_command.positions,
                dispatching_command.stiffnesses,
            ),
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::Initial => (
                self.current_minimizer
                    .optimize(context.sensor_data.currents, *context.initial_pose),
                Joints::fill(0.6),
            ),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::Penalized => (
                self.current_minimizer
                    .optimize(context.sensor_data.currents, *context.penalized_pose),
                Joints::fill(0.6),
            ),
            MotionType::SitDown => (sit_down.positions, sit_down.stiffnesses),
            MotionType::Stand => (
                self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                ),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::StandUpBack => (*stand_up_back_positions, Joints::fill(1.0)),
            MotionType::StandUpFront => (*stand_up_front_positions, Joints::fill(1.0)),
            MotionType::Unstiff => (current_positions, Joints::fill(0.0)),
            MotionType::Walk => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
        };

        // The actuators use the raw sensor data (not corrected like current_positions) in their feedback loops,
        // thus the compensation is required to make them reach the actual desired position.
        let compensated_positions = positions + *context.joint_calibration_offsets;
        let motor_commands = MotorCommands {
            positions: compensated_positions,
            stiffnesses,
        };

        context
            .motor_position_difference
            .fill_if_subscribed(|| motor_commands.positions - current_positions);

        context
            .penalized_current_minimizer
            .fill_if_subscribed(|| self.current_minimizer);

        Ok(MainOutputs {
            motor_commands: motor_commands.into(),
        })
    }
}
