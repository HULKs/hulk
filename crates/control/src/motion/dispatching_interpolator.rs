use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use motionfile::{SplineInterpolator, TimedSpline};
use types::{
    BodyJointsCommand, ConditionInput, CycleTime, HeadJoints, Joints, JointsCommand,
    JointsVelocity, MotionFinished, MotionSelection, MotionType, SensorData,
};

pub struct DispatchingInterpolator {
    interpolator: SplineInterpolator<Joints<f32>>,
    stiffnesses: Joints<f32>,
    last_currently_active: bool,
    last_dispatching_motion: MotionType,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub arms_up_squat_joints_command: Input<JointsCommand<f32>, "arms_up_squat_joints_command">,
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub energy_saving_stand: Input<BodyJointsCommand<f32>, "energy_saving_stand_command">,
    pub jump_left_joints_command: Input<JointsCommand<f32>, "jump_left_joints_command">,
    pub jump_right_joints_command: Input<JointsCommand<f32>, "jump_right_joints_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub sit_down_joints_command: Input<JointsCommand<f32>, "sit_down_joints_command">,
    pub stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    pub stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    pub walk_joints_command: Input<BodyJointsCommand<f32>, "walk_joints_command">,

    pub maximum_velocity: Parameter<JointsVelocity, "maximum_joint_velocities">,
    pub penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub motion_finished: PersistentState<MotionFinished, "motion_finished">,

    pub transition_time: AdditionalOutput<Option<Duration>, "transition_time">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub dispatching_command: MainOutput<JointsCommand<f32>>,
}

impl DispatchingInterpolator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: Default::default(),
            stiffnesses: Default::default(),
            last_currently_active: false,
            last_dispatching_motion: MotionType::Unstiff,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context.motion_finished[MotionType::Dispatching] = false;

        let currently_active = context.motion_selection.current_motion == MotionType::Dispatching;
        if !currently_active {
            context.transition_time.fill_if_subscribed(|| None);

            self.last_currently_active = currently_active;
            return Ok(Default::default());
        }
        let dispatching_motion = match context.motion_selection.dispatching_motion {
            Some(motion) => motion,
            None => return Ok(Default::default()),
        };
        let interpolator_reset_required =
            self.last_dispatching_motion != dispatching_motion || !self.last_currently_active;
        self.last_dispatching_motion = dispatching_motion;
        self.last_currently_active = currently_active;

        if interpolator_reset_required {
            let target_position = match dispatching_motion {
                MotionType::ArmsUpSquat => context.arms_up_squat_joints_command.positions,
                MotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                MotionType::FallProtection => panic!("Is executed immediately"),
                MotionType::JumpLeft => context.jump_left_joints_command.positions,
                MotionType::JumpRight => context.jump_right_joints_command.positions,
                MotionType::Penalized => *context.penalized_pose,
                MotionType::SitDown => context.sit_down_joints_command.positions,
                MotionType::Stand => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.walk_joints_command.positions,
                ),
                MotionType::StandUpBack => *context.stand_up_back_positions,
                MotionType::StandUpFront => *context.stand_up_front_positions,
                MotionType::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                MotionType::Walk => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.walk_joints_command.positions,
                ),
                MotionType::EnergySavingStand => Joints::from_head_and_body(
                    HeadJoints::fill(0.0),
                    context.energy_saving_stand.positions,
                ),
            };

            self.interpolator = TimedSpline::try_new_transition_timed(
                context.sensor_data.positions,
                target_position,
                Duration::from_secs_f32(1.0),
            )?
            .into();
            self.stiffnesses = Joints::fill(0.8);
        }

        self.interpolator
            .advance_by(context.cycle_time.last_cycle_duration);

        context.motion_finished[MotionType::Dispatching] = self.interpolator.is_finished();
        context.transition_time.fill_if_subscribed(|| {
            if self.interpolator.is_finished() {
                None
            } else {
                Some(self.interpolator.total_duration())
            }
        });

        Ok(MainOutputs {
            dispatching_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: self.stiffnesses,
            }
            .into(),
        })
    }
}
