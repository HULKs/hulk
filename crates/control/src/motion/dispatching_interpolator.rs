use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use motionfile::{SplineInterpolator, TimedSpline};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{body::BodyJoints, head::HeadJoints, Joints},
    motion_selection::{MotionSafeExits, MotionSelection, MotionType},
    motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
pub struct DispatchingInterpolator {
    interpolator: SplineInterpolator<Joints<f32>>,
    stiffnesses: Joints<f32>,
    was_dispatching: bool,
    last_dispatching_motion: MotionType,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    arms_up_squat_joints_command: Input<MotorCommands<Joints<f32>>, "arms_up_squat_joints_command">,
    jump_left_joints_command: Input<MotorCommands<Joints<f32>>, "jump_left_joints_command">,
    jump_right_joints_command: Input<MotorCommands<Joints<f32>>, "jump_right_joints_command">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    cycle_time: Input<CycleTime, "cycle_time">,
    sit_down_joints_command: Input<MotorCommands<Joints<f32>>, "sit_down_joints_command">,
    stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    stand_up_sitting_positions: Input<Joints<f32>, "stand_up_sitting_positions">,
    walk_motor_commands: Input<MotorCommands<BodyJoints<f32>>, "walk_motor_commands">,

    initial_pose: Parameter<Joints<f32>, "initial_pose">,
    penalized_pose: Parameter<Joints<f32>, "penalized_pose">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    last_actuated_motor_commands:
        CyclerState<MotorCommands<Joints<f32>>, "last_actuated_motor_commands">,

    transition_time: AdditionalOutput<Option<Duration>, "transition_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dispatching_command: MainOutput<MotorCommands<Joints<f32>>>,
}

impl DispatchingInterpolator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: Default::default(),
            stiffnesses: Default::default(),
            was_dispatching: false,
            last_dispatching_motion: MotionType::Unstiff,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context.motion_safe_exits[MotionType::Dispatching] = false;

        let dispatching = context.motion_selection.current_motion == MotionType::Dispatching;
        if !dispatching {
            context.transition_time.fill_if_subscribed(|| None);

            self.was_dispatching = false;
            return Ok(Default::default());
        }
        let dispatching_motion = match context.motion_selection.dispatching_motion {
            Some(motion) => motion,
            None => return Ok(Default::default()),
        };
        let interpolator_reset_required =
            self.last_dispatching_motion != dispatching_motion || !self.was_dispatching;
        self.last_dispatching_motion = dispatching_motion;
        self.was_dispatching = dispatching;

        if interpolator_reset_required {
            let target_position = match dispatching_motion {
                MotionType::ArmsUpSquat => context.arms_up_squat_joints_command.positions,
                MotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                MotionType::FallProtection => panic!("Is executed immediately"),
                MotionType::Initial => *context.initial_pose,
                MotionType::JumpLeft => context.jump_left_joints_command.positions,
                MotionType::JumpRight => context.jump_right_joints_command.positions,
                MotionType::Penalized => *context.penalized_pose,
                MotionType::SitDown => context.sit_down_joints_command.positions,
                MotionType::Stand => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.walk_motor_commands.positions,
                ),
                MotionType::StandUpBack => *context.stand_up_back_positions,
                MotionType::StandUpFront => *context.stand_up_front_positions,
                MotionType::StandUpSitting => *context.stand_up_sitting_positions,
                MotionType::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                MotionType::Animation => panic!("Dispatching Animation doesn't make sense"),
                MotionType::AnimationStiff => panic!("Dispatching AnimationStiff should not happen"),
                MotionType::Walk => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.walk_motor_commands.positions,
                ),
            };

            self.interpolator = TimedSpline::try_new_transition_timed(
                context.last_actuated_motor_commands.positions,
                target_position,
                Duration::from_secs_f32(1.0),
            )?
            .into();
            self.stiffnesses = Joints::fill(0.8);
        }

        self.interpolator
            .advance_by(context.cycle_time.last_cycle_duration);

        context.motion_safe_exits[MotionType::Dispatching] = self.interpolator.is_finished();
        context.transition_time.fill_if_subscribed(|| {
            if self.interpolator.is_finished() {
                None
            } else {
                Some(self.interpolator.total_duration())
            }
        });

        Ok(MainOutputs {
            dispatching_command: MotorCommands {
                positions: self.interpolator.value(),
                stiffnesses: self.stiffnesses,
            }
            .into(),
        })
    }
}
