use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use motionfile::{SplineInterpolator, TimedSpline};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{body::BodyJoints, head::HeadJoints, Joints},
    motion_file_player::MotionFileState,
    motion_selection::{MotionSafeExits, MotionSelection, MotionVariant},
    motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
pub struct DispatchingInterpolator {
    interpolator: SplineInterpolator<Joints<f32>>,
    stiffnesses: Joints<f32>,
    was_dispatching: bool,
    last_dispatching_motion: MotionVariant,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    arms_up_squat: Input<MotionFileState, "arms_up_squat">,
    jump_left: Input<MotionFileState, "jump_left">,
    jump_right: Input<MotionFileState, "jump_right">,
    sit_down: Input<MotionFileState, "sit_down">,
    stand_up_back: Input<MotionFileState, "stand_up_back">,
    stand_up_front: Input<MotionFileState, "stand_up_front">,
    stand_up_sitting: Input<MotionFileState, "stand_up_sitting">,
    stand_up_squatting: Input<MotionFileState, "stand_up_squatting">,

    motion_selection: Input<MotionSelection, "motion_selection">,
    cycle_time: Input<CycleTime, "cycle_time">,
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
            last_dispatching_motion: MotionVariant::Unstiff,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context.motion_safe_exits[MotionVariant::Dispatching] = false;

        let dispatching = context.motion_selection.current_motion == MotionVariant::Dispatching;
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
                MotionVariant::ArmsUpSquat => context.arms_up_squat.commands.positions,
                MotionVariant::Dispatching => panic!("Dispatching cannot dispatch itself"),
                MotionVariant::FallProtection => panic!("Is executed immediately"),
                MotionVariant::Initial => *context.initial_pose,
                MotionVariant::JumpLeft => context.jump_left.commands.positions,
                MotionVariant::JumpRight => context.jump_right.commands.positions,
                MotionVariant::Penalized => *context.penalized_pose,
                MotionVariant::SitDown => context.sit_down.commands.positions,
                MotionVariant::Stand => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.walk_motor_commands.positions,
                ),
                MotionVariant::StandUpBack => context.stand_up_back.commands.positions,
                MotionVariant::StandUpFront => context.stand_up_front.commands.positions,
                MotionVariant::StandUpSitting => context.stand_up_sitting.commands.positions,
                MotionVariant::StandUpSquatting => context.stand_up_squatting.commands.positions,
                MotionVariant::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                MotionVariant::Walk => Joints::from_head_and_body(
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

        context.motion_safe_exits[MotionVariant::Dispatching] = self.interpolator.is_finished();
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
