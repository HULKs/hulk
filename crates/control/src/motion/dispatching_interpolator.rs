use std::time::Duration;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    BodyJointsCommand, CycleTime, HeadJoints, Joints, JointsCommand, LinearInterpolator,
    MotionSafeExits, MotionSelection, MotionType, SensorData,
};

pub struct DispatchingInterpolator {
    interpolator: LinearInterpolator<Joints<f32>>,
    stiffnesses: Joints<f32>,
    last_currently_active: bool,
    last_dispatching_motion: MotionType,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub arms_up_squat_joints_command: Input<JointsCommand<f32>, "arms_up_squat_joints_command">,
    pub jump_left_joints_command: Input<JointsCommand<f32>, "jump_left_joints_command">,
    pub jump_right_joints_command: Input<JointsCommand<f32>, "jump_right_joints_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub sit_down_joints_command: Input<JointsCommand<f32>, "sit_down_joints_command">,
    pub stand_up_back_positions: Input<Joints<f32>, "stand_up_back_positions">,
    pub stand_up_front_positions: Input<Joints<f32>, "stand_up_front_positions">,
    pub walk_joints_command: Input<BodyJointsCommand<f32>, "walk_joints_command">,

    pub penalized_pose: Parameter<Joints<f32>, "penalized_pose">,
    pub ready_pose: Parameter<Joints<f32>, "ready_pose">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
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

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        context.motion_safe_exits[MotionType::Dispatching] = false;

        let currently_active = context.motion_selection.current_motion == MotionType::Dispatching;
        if !currently_active {
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
            (self.interpolator, self.stiffnesses) = match dispatching_motion {
                MotionType::ArmsUpSquat => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        context.arms_up_squat_joints_command.positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                MotionType::FallProtection => panic!("Is executed immediately"),
                MotionType::JumpLeft => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        context.jump_left_joints_command.positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::JumpRight => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        context.jump_right_joints_command.positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Penalized => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        *context.penalized_pose,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::SitDown => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        context.sit_down_joints_command.positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Stand => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        Joints::from_head_and_body(
                            HeadJoints::fill(0.0),
                            context.walk_joints_command.positions,
                        ),
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::StandUpBack => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        *context.stand_up_back_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::StandUpFront => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        *context.stand_up_front_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                MotionType::Walk => (
                    LinearInterpolator::new(
                        context.sensor_data.positions,
                        Joints::from_head_and_body(
                            HeadJoints::fill(0.0),
                            context.walk_joints_command.positions,
                        ),
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
            };
        }

        self.interpolator
            .step(context.cycle_time.last_cycle_duration);

        context.motion_safe_exits[MotionType::Dispatching] = self.interpolator.is_finished();

        Ok(MainOutputs {
            dispatching_command: JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: self.stiffnesses,
            }
            .into(),
        })
    }
}
