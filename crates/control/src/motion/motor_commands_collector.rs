use color_eyre::Result;
use context_attribute::context;
use energy_optimization::{current_minimizer::CurrentMinimizer, CurrentMinimizerParameters};
use framework::{AdditionalOutput, MainOutput};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{
        arm::ArmJoints,
        body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
        head::HeadJoints,
        leg::LegJoints,
        Joints,
    },
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
    animation_commands: Input<MotorCommands<Joints<f32>>, "animation_commands">,
    arms_up_squat_joints_command: Input<MotorCommands<Joints<f32>>, "arms_up_squat_joints_command">,
    arms_up_stand_joints_command: Input<MotorCommands<Joints<f32>>, "arms_up_stand_joints_command">,
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
    wide_stance_positions: Input<Joints<f32>, "wide_stance_positions">,
    wide_stance_left_positions: Input<Joints<f32>, "wide_stance_left_positions">,
    wide_stance_right_positions: Input<Joints<f32>, "wide_stance_right_positions">,
    center_jump_positions: Input<Joints<f32>, "center_jump_positions">,
    walk_motor_commands: Input<MotorCommands<BodyJoints<f32>>, "walk_motor_commands">,
    cycle_time: Input<CycleTime, "cycle_time">,
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
        let animation = context.animation_commands;
        let dispatching_command = context.dispatching_command;
        let fall_protection_positions = context.fall_protection_command.positions;
        let fall_protection_stiffnesses = context.fall_protection_command.stiffnesses;
        let head_joints_command = context.head_joints_command;
        let motion_selection = context.motion_selection;
        let arms_up_squat = context.arms_up_squat_joints_command;
        let arms_up_stand = context.arms_up_stand_joints_command;
        let jump_left = context.jump_left_joints_command;
        let jump_right = context.jump_right_joints_command;
        let center_jump_positions = context.center_jump_positions;
        let sit_down = context.sit_down_joints_command;
        let stand_up_back_positions = context.stand_up_back_positions;
        let stand_up_front_positions = context.stand_up_front_positions;
        let stand_up_sitting_positions = context.stand_up_sitting_positions;
        let wide_stance_positions = context.wide_stance_positions;
        let wide_stance_left_positions = context.wide_stance_left_positions;
        let wide_stance_right_positions = context.wide_stance_right_positions;
        let walk = context.walk_motor_commands;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::Animation => (animation.positions, animation.stiffnesses),
            MotionType::AnimationStiff => (animation.positions, animation.stiffnesses),
            MotionType::ArmsUpSquat => (arms_up_squat.positions, arms_up_squat.stiffnesses),
            MotionType::ArmsUpStand => (arms_up_stand.positions, arms_up_stand.stiffnesses),
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
                Joints::from_head_and_body(
                    HeadJoints::fill(0.6),
                    BodyJoints::from_lower_and_upper(
                        LowerBodyJoints::fill(0.6),
                        UpperBodyJoints::fill(0.01),
                    ),
                ),
            ),
            MotionType::JumpLeft => (jump_left.positions, jump_left.stiffnesses),
            MotionType::JumpRight => (jump_right.positions, jump_right.stiffnesses),
            MotionType::CenterJump => (
                *center_jump_positions,
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

            MotionType::Penalized => (
                self.current_minimizer.optimize(
                    context.sensor_data.currents,
                    *context.penalized_pose,
                    *context.cycle_time,
                    *context.current_minimizer_parameters,
                ),
                Joints::from_head_and_body(
                    HeadJoints::fill(0.6),
                    BodyJoints::from_lower_and_upper(
                        LowerBodyJoints::fill(0.6),
                        UpperBodyJoints::fill(0.01),
                    ),
                ),
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
            MotionType::WideStance => (
                *wide_stance_positions,
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
            MotionType::WideStanceLeft => (
                *wide_stance_left_positions,
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
            MotionType::WideStanceRight => (
                *wide_stance_right_positions,
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

        let motor_commands = MotorCommands {
            positions,
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
