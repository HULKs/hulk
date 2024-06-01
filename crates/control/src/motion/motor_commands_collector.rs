use color_eyre::Result;
use context_attribute::context;
use energy_optimization::{current_minimizer::CurrentMinimizer, CurrentMinimizerParameters};
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{arm::ArmJoints, body::BodyJoints, head::HeadJoints, leg::LegJoints, Joints},
    motion_selection::{MotionSelection, MotionType},
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
    animation_positions: Input<MotorCommands<Joints<f32>>, "animation_positions">,
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
    stand_up_sitting_positions: Input<Joints<f32>, "stand_up_sitting_positions">,
    walk_motor_commands: Input<MotorCommands<BodyJoints<f32>>, "walk_motor_commands">,
    cycle_time: Input<CycleTime, "cycle_time">,

    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    initial_pose: Parameter<Joints<f32>, "initial_pose">,
    current_minimizer_parameters:
        Parameter<CurrentMinimizerParameters, "current_minimizer_parameters">,
    stand_up_stiffness_upper_body: Parameter<f32, "stand_up_stiffness_upper_body">,

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
        let current_positions = context.sensor_data.positions;
        let animation_positions = context.animation_positions;    
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
        let stand_up_sitting_positions = context.stand_up_sitting_positions;
        let walk = context.walk_motor_commands;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::Animation => (current_positions, Joints::fill(0.0)),
            MotionType::AnimationStiff => (current_positions, Joints::fill(1.0)),
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::Dispatching => {
                self.current_minimizer.reset();
                (
                    dispatching_command.positions,
                    dispatching_command.stiffnesses,
                )
            }
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::Initial => (
                self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    Joints::from_head_and_body(
                        head_joints_command.positions,
                        context.initial_pose.body(),
                    ),
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                Joints::fill(0.6),
            ),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::Penalized => (
                self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    *context.penalized_pose,
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                Joints::fill(0.6),
            ),
            MotionType::SitDown => (sit_down.positions, sit_down.stiffnesses),
            MotionType::Stand => (
                self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::StandUpBack => (
                *stand_up_back_positions,
                Joints::from_head_and_body(
                    HeadJoints::fill(*context.stand_up_stiffness_upper_body),
                    BodyJoints {
                        left_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        right_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        left_leg: LegJoints::fill(1.0),
                        right_leg: LegJoints::fill(1.0),
                    },
                ),
            ),
            MotionType::StandUpFront => (
                *stand_up_front_positions,
                Joints::from_head_and_body(
                    HeadJoints::fill(*context.stand_up_stiffness_upper_body),
                    BodyJoints {
                        left_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        right_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        left_leg: LegJoints::fill(1.0),
                        right_leg: LegJoints::fill(1.0),
                    },
                ),
            ),
            MotionType::StandUpSitting => (
                *stand_up_sitting_positions,
                Joints::from_head_and_body(
                    HeadJoints::fill(*context.stand_up_stiffness_upper_body),
                    BodyJoints {
                        left_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        right_arm: ArmJoints::fill(*context.stand_up_stiffness_upper_body),
                        left_leg: LegJoints::fill(1.0),
                        right_leg: LegJoints::fill(1.0),
                    },
                ),
            ),
            MotionType::Unstiff => (measured_positions, Joints::fill(0.0)),
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
            .fill_if_subscribed(|| motor_commands.positions - measured_positions);

        context
            .current_minimizer
            .fill_if_subscribed(|| self.current_minimizer);

        Ok(MainOutputs {
            motor_commands: motor_commands.into(),
        })
    }
}
