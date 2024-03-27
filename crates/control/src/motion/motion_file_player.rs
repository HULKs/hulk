use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use motionfile::{MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use std::fmt::Debug;
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::{mirror::Mirror, Joints},
    motion_file_player::MotionFileState,
    motion_selection::{MotionSafeExits, MotionSelection, MotionVariant},
    motor_commands::MotorCommands,
};

#[derive(Deserialize, Serialize)]
struct Motion<T> {
    variant: MotionVariant,
    interpolator: MotionInterpolator<T>,
}

impl<T> Motion<T>
where
    for<'de> T: Debug + Interpolate<f32> + Deserialize<'de> + Default,
{
    pub fn try_new(
        paths: &impl PathsInterface,
        file_name: &str,
        variant: MotionVariant,
    ) -> Result<Self> {
        Ok(Self {
            variant,
            interpolator: MotionFile::<T>::from_path(paths.get_paths().motions.join(file_name))?
                .try_into()?,
        })
    }

    pub fn cycle(&mut self, context: &mut CycleContext) {
        if context.motion_selection.current_motion == self.variant {
            self.advance(context);
        } else {
            self.interpolator.reset();
            context.motion_safe_exits[self.variant] = false;
        };
    }

    pub fn advance(&mut self, context: &mut CycleContext) {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let condition_input = context.condition_input;

        self.interpolator
            .advance_by(last_cycle_duration, condition_input);

        context.motion_safe_exits[self.variant] = self.interpolator.is_finished();
    }
}

impl Motion<MotorCommands<Joints<f32>>> {
    pub fn state(&self) -> MotionFileState {
        MotionFileState {
            commands: self.interpolator.value(),
            remaining_duration: self.interpolator.estimated_remaining_duration(),
        }
    }
}

impl Motion<Joints<f32>> {
    pub fn state(&self, stiffness: f32) -> MotionFileState {
        let commands = MotorCommands {
            positions: self.interpolator.value(),
            stiffnesses: Joints::fill(stiffness),
        };
        MotionFileState {
            commands,
            remaining_duration: self.interpolator.estimated_remaining_duration(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct MotionFilePlayer {
    arms_up_squat: Motion<Joints<f32>>,
    jump_left: Motion<MotorCommands<Joints<f32>>>,
    jump_right: Motion<MotorCommands<Joints<f32>>>,
    sit_down: Motion<Joints<f32>>,
    stand_up_back: Motion<Joints<f32>>,
    stand_up_front: Motion<Joints<f32>>,
    stand_up_sitting: Motion<Joints<f32>>,
    stand_up_squatting: Motion<Joints<f32>>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    condition_input: Input<ConditionInput, "condition_input">,
    cycle_time: Input<CycleTime, "cycle_time">,
    motion_selection: Input<MotionSelection, "motion_selection">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub arms_up_squat: MainOutput<MotionFileState>,
    pub jump_left: MainOutput<MotionFileState>,
    pub jump_right: MainOutput<MotionFileState>,
    pub sit_down: MainOutput<MotionFileState>,
    pub stand_up_back: MainOutput<MotionFileState>,
    pub stand_up_front: MainOutput<MotionFileState>,
    pub stand_up_sitting: MainOutput<MotionFileState>,
    pub stand_up_squatting: MainOutput<MotionFileState>,
}

impl MotionFilePlayer {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        Ok(Self {
            arms_up_squat: Motion::try_new(
                &**context.hardware_interface,
                "arms_up_squat.json",
                MotionVariant::ArmsUpSquat,
            )?,
            jump_left: Motion::try_new(
                &**context.hardware_interface,
                "jump_left.json",
                MotionVariant::JumpLeft,
            )?,
            jump_right: Motion::try_new(
                &**context.hardware_interface,
                "jump_left.json",
                MotionVariant::JumpRight,
            )?,
            sit_down: Motion::try_new(
                &**context.hardware_interface,
                "sit_down.json",
                MotionVariant::SitDown,
            )?,
            stand_up_back: Motion::try_new(
                &**context.hardware_interface,
                "stand_up_back.json",
                MotionVariant::StandUpBack,
            )?,
            stand_up_front: Motion::try_new(
                &**context.hardware_interface,
                "stand_up_front.json",
                MotionVariant::StandUpFront,
            )?,
            stand_up_sitting: Motion::try_new(
                &**context.hardware_interface,
                "stand_up_sitting.json",
                MotionVariant::StandUpSitting,
            )?,
            stand_up_squatting: Motion::try_new(
                &**context.hardware_interface,
                "stand_up_squatting.json",
                MotionVariant::StandUpSquatting,
            )?,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.arms_up_squat.cycle(&mut context);
        self.jump_left.cycle(&mut context);
        self.jump_right.cycle(&mut context);
        self.sit_down.cycle(&mut context);
        self.stand_up_back.cycle(&mut context);
        self.stand_up_front.cycle(&mut context);
        self.stand_up_sitting.cycle(&mut context);
        self.stand_up_squatting.cycle(&mut context);

        Ok(MainOutputs {
            arms_up_squat: self.arms_up_squat.state(0.8).into(),
            jump_left: self.jump_left.state().into(),
            jump_right: {
                let mut state = self.jump_right.state();
                state.commands = state.commands.mirrored();
                state
            }
            .into(),
            sit_down: self.sit_down.state(0.8).into(),
            stand_up_back: self.stand_up_back.state(0.8).into(),
            stand_up_front: self.stand_up_front.state(1.0).into(),
            stand_up_sitting: self.stand_up_sitting.state(0.8).into(),
            stand_up_squatting: self.stand_up_squatting.state(0.8).into(),
        })
    }
}
