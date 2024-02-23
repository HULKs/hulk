use std::{ops::Range, time::Duration};

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    fall_state::{FallDirection, FallState, Side},
    joints::{
        arm::ArmJoints, body::BodyJoints, head::HeadJoints, leg::LegJoints, mirror::Mirror, Joints,
    },
    motion_selection::{MotionSafeExits, MotionType},
    motor_commands::MotorCommands,
    sensor_data::SensorData,
};

#[derive(Clone, Copy, Debug)]
enum FallPhase {
    Early,
    Late,
}

#[derive(Default, Serialize, Deserialize)]
pub struct FallProtector {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    fall_state: Input<FallState, "fall_state">,
    sensor_data: Input<SensorData, "sensor_data">,

    front_early: Parameter<Joints<f32>, "fall_protection.front_early">,
    front_late: Parameter<Joints<f32>, "fall_protection.front_late">,
    back_early: Parameter<Joints<f32>, "fall_protection.back_early">,
    back_late: Parameter<Joints<f32>, "fall_protection.back_late">,

    early_protection_timeout: Parameter<Duration, "fall_protection.early_protection_timeout">,
    reached_threshold: Parameter<f32, "fall_protection.reached_threshold">,
    head_stiffness: Parameter<Range<f32>, "fall_protection.head_stiffness">,
    arm_stiffness: Parameter<Range<f32>, "fall_protection.arm_stiffness">,
    leg_stiffness: Parameter<Range<f32>, "fall_protection.leg_stiffness">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<MotorCommands<Joints<f32>>>,
}

impl FallProtector {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let measured_positions = context.sensor_data.positions;

        let (start_time, falling_direction) = match *context.fall_state {
            FallState::Upright | FallState::Sitting { .. } | FallState::Fallen { .. } => {
                context.motion_safe_exits[MotionType::FallProtection] = true;
                return Ok(MainOutputs::default());
            }
            FallState::Falling {
                start_time,
                direction,
            } => (start_time, direction),
        };
        context.motion_safe_exits[MotionType::FallProtection] = false;

        let phase = if context
            .cycle_time
            .start_time
            .duration_since(start_time)
            .unwrap()
            < *context.early_protection_timeout
        {
            FallPhase::Early
        } else {
            FallPhase::Late
        };

        let protection_angles = match (falling_direction, phase) {
            (FallDirection::Forward { side: Side::Left }, FallPhase::Early) => {
                prevent_stuck_arms(context.front_early.mirrored(), measured_positions)
            }
            (FallDirection::Forward { side: Side::Left }, FallPhase::Late) => {
                prevent_stuck_arms(context.front_late.mirrored(), measured_positions)
            }
            (FallDirection::Forward { side: Side::Right }, FallPhase::Early) => {
                prevent_stuck_arms(*context.front_early, measured_positions)
            }
            (FallDirection::Forward { side: Side::Right }, FallPhase::Late) => {
                prevent_stuck_arms(*context.front_late, measured_positions)
            }
            (FallDirection::Backward { side: Side::Left }, FallPhase::Early) => {
                context.back_early.mirrored()
            }
            (FallDirection::Backward { side: Side::Left }, FallPhase::Late) => {
                context.back_late.mirrored()
            }
            (FallDirection::Backward { side: Side::Right }, FallPhase::Early) => {
                *context.back_early
            }
            (FallDirection::Backward { side: Side::Right }, FallPhase::Late) => *context.back_late,
        };

        let is_head_protected = measured_positions.head.pitch.abs() < *context.reached_threshold
            && measured_positions.head.yaw.abs() < *context.reached_threshold;

        let head_stiffnesses = if is_head_protected {
            HeadJoints::fill(context.head_stiffness.end)
        } else {
            HeadJoints::fill(context.head_stiffness.start)
        };

        let body_stiffnesses = match phase {
            FallPhase::Early => BodyJoints {
                left_arm: ArmJoints::fill(context.arm_stiffness.start),
                right_arm: ArmJoints::fill(context.arm_stiffness.start),
                left_leg: LegJoints::fill(context.leg_stiffness.start),
                right_leg: LegJoints::fill(context.leg_stiffness.start),
            },
            FallPhase::Late => BodyJoints {
                left_arm: ArmJoints::fill(context.arm_stiffness.end),
                right_arm: ArmJoints::fill(context.arm_stiffness.end),
                left_leg: LegJoints::fill(context.leg_stiffness.end),
                right_leg: LegJoints::fill(context.leg_stiffness.end),
            },
        };

        let motor_commands = MotorCommands {
            positions: protection_angles,
            stiffnesses: Joints::from_head_and_body(head_stiffnesses, body_stiffnesses),
        };

        Ok(MainOutputs {
            fall_protection_command: motor_commands.into(),
        })
    }
}

fn prevent_stuck_arms(request: Joints<f32>, measured_positions: Joints<f32>) -> Joints<f32> {
    let left_arm = if measured_positions.left_arm.shoulder_roll < 0.0
        && measured_positions.left_arm.shoulder_pitch > 1.6
    {
        ArmJoints {
            shoulder_pitch: 0.0,
            shoulder_roll: 0.35,
            elbow_yaw: 0.0,
            elbow_roll: 0.0,
            wrist_yaw: 0.0,
            hand: 0.0,
        }
    } else {
        request.left_arm
    };
    let right_arm = if measured_positions.right_arm.shoulder_roll > 0.0
        && measured_positions.right_arm.shoulder_pitch > 1.6
    {
        ArmJoints {
            shoulder_pitch: 0.0,
            shoulder_roll: -0.35,
            elbow_yaw: 0.0,
            elbow_roll: 0.0,
            wrist_yaw: 0.0,
            hand: 0.0,
        }
    } else {
        request.right_arm
    };
    Joints {
        left_arm,
        right_arm,
        ..request
    }
}
