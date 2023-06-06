use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use types::{
    BodyJointsCommand, HeadJoints, HeadJointsCommand, Joints, JointsCommand, Leds, MotionSafeExits,
    MotionSelection, MotionType, SensorData,
};

pub struct JointCommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub positions: AdditionalOutput<Joints<f32>, "positions">,
    pub compensated_positions: AdditionalOutput<Joints<f32>, "compensated_positions">,
    pub positions_difference: AdditionalOutput<Joints<f32>, "positions_difference">,
    pub stiffnesses: AdditionalOutput<Joints<f32>, "stiffnesses">,
    pub motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,

    pub center_head_position: Parameter<HeadJoints<f32>, "center_head_position">,
    pub joint_calibration_offsets: Parameter<Joints<f32>, "joint_calibration_offsets">,
    pub penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub arms_up_squat_joints_command: Input<JointsCommand<f32>, "arms_up_squat_joints_command">,
    pub dispatching_command: Input<JointsCommand<f32>, "dispatching_command">,
    pub energy_saving_stand_command: Input<BodyJointsCommand<f32>, "energy_saving_stand_command">,
    pub fall_protection_command: Input<JointsCommand<f32>, "fall_protection_command">,
    pub head_joints_command: Input<HeadJointsCommand<f32>, "head_joints_command">,
    pub jump_left_joints_command: Input<JointsCommand<f32>, "jump_left_joints_command">,
    pub jump_right_joints_command: Input<JointsCommand<f32>, "jump_right_joints_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub sit_down_joints_command: Input<JointsCommand<f32>, "sit_down_joints_command">,
    pub stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    pub stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    pub walk_joints_command: Input<BodyJointsCommand<f32>, "walk_joints_command">,
    pub hardware_interface: HardwareInterface,
    pub leds: Input<Leds, "leds">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl JointCommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl ActuatorInterface>,
    ) -> Result<MainOutputs> {
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

        // The actuators uses the raw sensor data (not corrected like current_positions) in their feedback loops,
        // thus the compensation is required to make them reach the actual desired position.
        let compensated_positions = positions + *context.joint_calibration_offsets;

        context
            .hardware_interface
            .write_to_actuators(compensated_positions, stiffnesses, *context.leds)
            .wrap_err("failed to write to actuators")?;

        context.positions.fill_if_subscribed(|| positions);

        context
            .compensated_positions
            .fill_if_subscribed(|| compensated_positions);

        context
            .positions_difference
            .fill_if_subscribed(|| positions - current_positions);
        context.stiffnesses.fill_if_subscribed(|| stiffnesses);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());
        Ok(MainOutputs {})
    }
}
