use macros::{module, require_some};

use crate::types::{
    BodyJoints, HeadJointsCommand, JointsCommand, MotionType, SensorData, SitDownJoints,
};

use crate::types::{HeadJoints, Joints, MotionSelection, WalkPositions};

pub struct JointCommandSender;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = dispatching_positions, data_type = Joints)]
#[input(path = sit_down_joints, data_type = SitDownJoints)]
#[input(path = stand_up_back_positions, data_type = Joints)]
#[input(path = stand_up_front_positions, data_type = Joints)]
#[input(path = walk_positions, data_type = WalkPositions)]
#[input(path = head_joints_command, data_type = HeadJointsCommand)]
#[input(path = fall_protection_command, data_type = JointsCommand)]
#[parameter(path = control.penalized_pose, data_type = Joints)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
#[main_output(name = positions, data_type = Joints)]
#[main_output(name = stiffnesses, data_type = Joints)]
impl JointCommandSender {}

impl JointCommandSender {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let current_positions = require_some!(context.sensor_data).positions;
        let dispatching_positions = require_some!(context.dispatching_positions);
        let fall_protection_positions = require_some!(context.fall_protection_command).positions;
        let fall_protection_stiffnesses =
            require_some!(context.fall_protection_command).stiffnesses;
        let head_joints_command = require_some!(context.head_joints_command);
        let motion_selection = require_some!(context.motion_selection);
        let sit_down_positions = require_some!(context.sit_down_joints).positions;
        let sit_down_stiffnesses = require_some!(context.sit_down_joints).stiffnesses;
        let stand_up_back_positions = require_some!(context.stand_up_back_positions);
        let stand_up_front_positions = require_some!(context.stand_up_front_positions);
        let walk_positions = require_some!(context.walk_positions).positions;

        let (positions, stiffnesses) = match motion_selection.current_motion {
            MotionType::Dispatching => (*dispatching_positions, Joints::fill(0.8)),
            MotionType::FallProtection => (fall_protection_positions, fall_protection_stiffnesses),
            MotionType::Jump => todo!(),
            MotionType::Kick => todo!(),
            MotionType::Penalized => (*context.penalized_pose, Joints::fill(0.8)),
            MotionType::SitDown => (sit_down_positions, sit_down_stiffnesses),
            MotionType::Stand => (
                Joints::from_head_and_body(head_joints_command.positions, walk_positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, BodyJoints::fill(0.8)),
            ),
            MotionType::StandUpBack => (*stand_up_back_positions, Joints::fill(0.8)),
            MotionType::StandUpFront => (*stand_up_front_positions, Joints::fill(0.8)),
            MotionType::Unstiff => (current_positions, Joints::fill(0.0)),
            MotionType::Walk => (
                Joints::from_head_and_body(head_joints_command.positions, walk_positions),
                Joints::from_head_and_body(head_joints_command.stiffnesses, BodyJoints::fill(0.8)),
            ),
        };

        Ok(MainOutputs {
            positions: Some(positions),
            stiffnesses: Some(stiffnesses),
        })
    }
}
