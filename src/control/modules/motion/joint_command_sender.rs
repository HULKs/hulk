use macros::{module, require_some};

use crate::types::{
    BodyMotionType, DispatchingHeadPositions, FallProtection, HeadMotionType, SensorData,
    SitDownPositions, StandUpBackPositions, StandUpFrontPositions,
};

use crate::types::{
    BodyJoints, DispatchingBodyPositions, HeadJoints, Joints, MotionSelection, WalkPositions,
};

pub struct JointCommandSender;

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[input(path = motion_selection, data_type = MotionSelection)]
#[input(path = dispatching_body_positions, data_type = DispatchingBodyPositions)]
#[input(path = dispatching_head_positions, data_type = DispatchingHeadPositions)]
#[input(path = sit_down_positions, data_type = SitDownPositions)]
#[input(path = stand_up_back_positions, data_type = StandUpBackPositions)]
#[input(path = stand_up_front_positions, data_type = StandUpFrontPositions)]
#[input(path = walk_positions, data_type = WalkPositions)]
#[input(path = look_around, data_type = HeadJoints)]
#[input(path = look_at, data_type = HeadJoints)]
#[input(path = zero_angles_head, data_type = HeadJoints)]
#[input(path = fall_protection, data_type = FallProtection)]
#[parameter(path = control.penalized_pose, data_type = Joints)]
#[parameter(path = control.ready_pose, data_type = Joints)]
#[parameter(path = control.center_head_position, data_type = HeadJoints)]
#[main_output(name = positions, data_type = Joints)]
#[main_output(name = stiffnesses, data_type = Joints)]
impl JointCommandSender {}

impl JointCommandSender {
    pub fn new() -> Self {
        Self
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let current_positions = require_some!(context.sensor_data).positions;
        let motion_selection = require_some!(context.motion_selection);
        let dispatching_body_positions =
            require_some!(context.dispatching_body_positions).positions;
        let dispatching_head_positions =
            require_some!(context.dispatching_head_positions).positions;
        let sit_down_positions = require_some!(context.sit_down_positions).positions;
        let sit_down_stiffnesses = require_some!(context.sit_down_positions).stiffnesses;
        let stand_up_back_positions = require_some!(context.stand_up_back_positions).positions;
        let stand_up_front_positions = require_some!(context.stand_up_front_positions).positions;
        let walk_positions = require_some!(context.walk_positions).positions;
        let look_around = require_some!(context.look_around);
        let look_at = require_some!(context.look_at);
        let zero_angles_head = require_some!(context.zero_angles_head);
        let fall_protection_head_position = require_some!(context.fall_protection).head_position;
        let fall_protection_head_stiffness = require_some!(context.fall_protection).head_stiffness;

        let (head_positions, head_stiffnesses) = match motion_selection.current_head_motion {
            HeadMotionType::Center => (*context.center_head_position, HeadJoints::fill(0.8)),
            HeadMotionType::FallProtection => (
                fall_protection_head_position,
                HeadJoints::fill(fall_protection_head_stiffness),
            ),
            HeadMotionType::Dispatching => (dispatching_head_positions, HeadJoints::fill(0.8)),
            HeadMotionType::LookAround => (*look_around, HeadJoints::fill(0.8)),
            HeadMotionType::LookAt => (*look_at, HeadJoints::fill(0.8)),
            HeadMotionType::Unstiff => (current_positions.into(), HeadJoints::fill(0.0)),
            HeadMotionType::ZeroAngles => (*zero_angles_head, HeadJoints::fill(0.8)),
        };

        let (body_positions, body_stiffnesses) = match motion_selection.current_body_motion {
            BodyMotionType::Dispatching => (dispatching_body_positions, BodyJoints::fill(0.8)),
            BodyMotionType::FallProtection => (current_positions.into(), BodyJoints::fill(0.0)),
            BodyMotionType::Jump => todo!(),
            BodyMotionType::Kick => todo!(),
            BodyMotionType::Penalized => (
                BodyJoints::from(*context.penalized_pose),
                BodyJoints::fill(0.8),
            ),
            BodyMotionType::SitDown => (sit_down_positions, sit_down_stiffnesses),
            BodyMotionType::Stand => (BodyJoints::from(*context.ready_pose), BodyJoints::fill(0.8)),
            BodyMotionType::StandUpBack => (stand_up_back_positions, BodyJoints::fill(0.8)),
            BodyMotionType::StandUpFront => (stand_up_front_positions, BodyJoints::fill(0.8)),
            BodyMotionType::Unstiff => (current_positions.into(), BodyJoints::fill(0.0)),
            BodyMotionType::Walk => (walk_positions, BodyJoints::fill(0.8)),
        };

        let positions = Joints::from_head_and_body(head_positions, body_positions);
        let stiffnesses = Joints::from_head_and_body(head_stiffnesses, body_stiffnesses);

        Ok(MainOutputs {
            positions: Some(positions),
            stiffnesses: Some(stiffnesses),
        })
    }
}
