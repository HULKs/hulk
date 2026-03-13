use std::{
    f32::consts::PI,
    time::{Duration, SystemTime},
};

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::AdditionalOutput;
use hardware::{HighLevelInterface, MotionRuntimeInteface};
use linear_algebra::{Orientation2, Point2};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    motion_command::{MotionCommand, OrientationMode},
    motion_runtime::MotionRuntime,
    parameters::RLWalkingParameters,
    path::traits::{Length, PathProgress},
    step::Step,
};

#[derive(Deserialize, Serialize)]
pub struct BoosterWalking {
    last_move_robot_time: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    cycle_time: Input<CycleTime, "cycle_time">,
    motion_command: Input<MotionCommand, "WorldState", "motion_command">,

    parameters: Parameter<RLWalkingParameters, "rl_walking">,
    move_robot_message_interval: Parameter<Duration, "motion.booster.move_robot_message_interval">,

    step: AdditionalOutput<Step, "additional_outputs.walking_step">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterWalking {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_move_robot_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }
        let parameters = context.parameters;
        let step = match context.motion_command {
            MotionCommand::Walk {
                path,
                orientation_mode,
                target_orientation,
                distance_to_be_aligned,
                speed,
                ..
            } => {
                let forward = path.forward(Point2::origin());
                let distance_to_target = path.length();
                let deceleration_factor =
                    (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
                let velocity = forward * *speed * deceleration_factor;

                let (walk_orientation, _tolerance): (Orientation2<Ground>, f32) =
                    match orientation_mode {
                        OrientationMode::Unspecified => todo!(),
                        OrientationMode::AlignWithPath => (Orientation2::from_vector(forward), 0.0),
                        OrientationMode::LookTowards {
                            direction,
                            tolerance,
                        } => (*direction, *tolerance),
                        OrientationMode::LookAt { target, tolerance } => (
                            Orientation2::from_vector(target - Point2::origin()),
                            *tolerance,
                        ),
                    };

                let target_alignment_importance = target_alignment_importance(
                    *distance_to_be_aligned,
                    parameters.hybrid_align_distance,
                    distance_to_target,
                );

                let orientation =
                    walk_orientation.slerp(*target_orientation, target_alignment_importance);

                let angular_velocity =
                    orientation.as_unit_vector().y() * parameters.max_alignment_rate;
                Step {
                    forward: velocity.x(),
                    left: velocity.y(),
                    turn: angular_velocity,
                }
            }
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => Step {
                forward: velocity.x(),
                left: velocity.y(),
                turn: *angular_velocity,
            },
            MotionCommand::Stand { .. } => Step::ZERO,
            _ => Step::ZERO,
        };

        context.step.fill_if_subscribed(|| step);

        if context
            .cycle_time
            .start_time
            .duration_since(self.last_move_robot_time)
            .expect("Time ran backwards")
            > *context.move_robot_message_interval
        {
            move_robot(&context, step);
        };

        Ok(MainOutputs {})
    }
}

fn move_robot(context: &CycleContext<impl HighLevelInterface + MotionRuntimeInteface>, step: Step) {
    let _ = context
        .hardware_interface
        .move_robot(step)
        .inspect_err(|err| log::error!("{err:?}"));
}

// https://www.desmos.com/calculator/ng03egi9mp
fn target_alignment_importance(
    distance_to_be_aligned: f32,
    hybrid_align_distance: f32,
    distance_to_target: f32,
) -> f32 {
    if distance_to_target < distance_to_be_aligned {
        1.0
    } else if distance_to_target < distance_to_be_aligned + hybrid_align_distance {
        (1.0 + f32::cos(PI * (distance_to_target - distance_to_be_aligned) / hybrid_align_distance))
            * 0.5
    } else {
        0.0
    }
}
