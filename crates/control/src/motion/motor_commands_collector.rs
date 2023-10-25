use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    joints::Joints,
    motion_selection::{MotionSelection, MotionType},
    motor_commands::{BodyMotorCommands, HeadMotorCommands, MotorCommands},
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct MotorCommandCollector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    arms_up_squat_joints_command: Input<MotorCommands<f32>, "arms_up_squat_joints_command">,
    dispatching_command: Input<MotorCommands<f32>, "dispatching_command">,
    energy_saving_stand_command: Input<BodyMotorCommands<f32>, "energy_saving_stand_command">,
    fall_protection_command: Input<MotorCommands<f32>, "fall_protection_command">,
    head_joints_command: Input<HeadMotorCommands<f32>, "head_joints_command">,
    jump_left_joints_command: Input<MotorCommands<f32>, "jump_left_joints_command">,
    jump_right_joints_command: Input<MotorCommands<f32>, "jump_right_joints_command">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    sensor_data: Input<SensorData, "sensor_data">,
    sit_down_joints_command: Input<MotorCommands<f32>, "sit_down_joints_command">,
    stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    walk_joints_command: Input<BodyMotorCommands<f32>, "walk_joints_command">,

    joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    initial_pose: Parameter<Joints<f32>, "initial_pose">,

    motor_position_difference: AdditionalOutput<Joints<f32>, "motor_positions_difference">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub motor_commands: MainOutput<MotorCommands<f32>>,
}

impl MotorCommandCollector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
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
        let walk = context.walk_joints_command;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::Dispatching => (
                dispatching_command.positions,
                dispatching_command.stiffnesses,
            ),
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::Initial => (*context.initial_pose, Joints::fill(0.8)),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::Penalized => (*context.penalized_pose, Joints::fill(0.8)),
            MotionType::SitDown => (sit_down.positions, sit_down.stiffnesses),
            MotionType::Stand => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::StandUpBack => (*stand_up_back_positions, Joints::fill(1.0)),
            MotionType::StandUpFront => (*stand_up_front_positions, Joints::fill(1.0)),
            MotionType::Unstiff => (current_positions, Joints::fill(0.0)),
            MotionType::Walk => (
                Joints::from_head_and_body(head_joints_command.positions, walk.positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, walk.stiffnesses),
            ),
            MotionType::EnergySavingStand => (
                Joints::from_head_and_body(
                    head_joints_command.positions,
                    context.energy_saving_stand_command.positions,
                ),
                Joints::from_head_and_body(
                    head_joints_command.stiffnesses,
                    context.energy_saving_stand_command.stiffnesses,
                ),
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

        Ok(MainOutputs {
            motor_commands: motor_commands.into(),
        })
    }
}
