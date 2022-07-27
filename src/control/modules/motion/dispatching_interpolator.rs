use std::time::Duration;

use anyhow::Result;
use module_derive::{module, require_some};
use types::{
    BodyJointsCommand, HeadJoints, Joints, JointsCommand, MotionSafeExits, MotionSelection,
    MotionType, SensorData,
};

use crate::control::linear_interpolator::LinearInterpolator;

pub struct DispatchingInterpolator {
    interpolator: LinearInterpolator<Joints>,
    stiffnesses: Joints,
    last_currently_active: bool,
    last_dispatching_motion: MotionType,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = arms_up_squat_joints_command, data_type = JointsCommand)]
#[input(path = jump_left_joints_command, data_type = JointsCommand)]
#[input(path = jump_right_joints_command, data_type = JointsCommand)]
#[input(path = stand_up_back_positions, data_type = Joints)]
#[input(path = stand_up_front_positions, data_type = Joints)]
#[input(path = sit_down_joints_command, data_type = JointsCommand)]
#[input(path = walk_joints_command, data_type = BodyJointsCommand)]
#[persistent_state(path = motion_safe_exits, data_type = MotionSafeExits)]
#[parameter(path = control.penalized_pose, data_type = Joints)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[main_output(name = dispatching_command, data_type = JointsCommand)]
impl DispatchingInterpolator {}

impl DispatchingInterpolator {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            interpolator: Default::default(),
            stiffnesses: Default::default(),
            last_currently_active: false,
            last_dispatching_motion: MotionType::Unstiff,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        context.motion_safe_exits[MotionType::Dispatching] = false;

        let sensor_data = require_some!(context.sensor_data);
        let motion_selection = require_some!(context.motion_selection);

        let currently_active = motion_selection.current_motion == MotionType::Dispatching;
        if !currently_active {
            self.last_currently_active = currently_active;
            return Ok(MainOutputs {
                dispatching_command: Some(Default::default()),
            });
        }

        let dispatching_motion = require_some!(motion_selection.dispatching_motion);
        let arms_up_squat_positions = require_some!(context.arms_up_squat_joints_command).positions;
        let jump_left_positions = require_some!(context.jump_left_joints_command).positions;
        let jump_right_positions = require_some!(context.jump_right_joints_command).positions;
        let stand_up_back_positions = require_some!(context.stand_up_back_positions);
        let stand_up_front_positions = require_some!(context.stand_up_front_positions);
        let walk_positions = require_some!(context.walk_joints_command).positions;
        let sit_down_positions = require_some!(context.sit_down_joints_command).positions;

        let interpolator_reset_required =
            self.last_dispatching_motion != dispatching_motion || !self.last_currently_active;
        self.last_dispatching_motion = dispatching_motion;
        self.last_currently_active = currently_active;

        if interpolator_reset_required {
            (self.interpolator, self.stiffnesses) = match dispatching_motion {
                MotionType::ArmsUpSquat => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        arms_up_squat_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Dispatching => panic!("Dispatching cannot dispatch itself"),
                MotionType::FallProtection => panic!("Is executed immediately"),
                MotionType::JumpLeft => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        jump_left_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::JumpRight => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        jump_right_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Penalized => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        *context.penalized_pose,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::SitDown => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        sit_down_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Stand => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        Joints::from_head_and_body(HeadJoints::fill(0.0), walk_positions),
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::StandUpBack => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        *stand_up_back_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::StandUpFront => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        *stand_up_front_positions,
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
                MotionType::Unstiff => panic!("Dispatching Unstiff doesn't make sense"),
                MotionType::Walk => (
                    LinearInterpolator::new(
                        sensor_data.positions,
                        Joints::from_head_and_body(HeadJoints::fill(0.0), walk_positions),
                        Duration::from_secs(1),
                    ),
                    Joints::fill(0.8),
                ),
            };
        }

        self.interpolator
            .step(sensor_data.cycle_info.last_cycle_duration);

        context.motion_safe_exits[MotionType::Dispatching] = self.interpolator.is_finished();

        Ok(MainOutputs {
            dispatching_command: Some(JointsCommand {
                positions: self.interpolator.value(),
                stiffnesses: self.stiffnesses,
            }),
        })
    }
}
