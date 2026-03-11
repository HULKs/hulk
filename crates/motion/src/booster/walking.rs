use std::f32::consts::PI;

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use hardware::{HighLevelInterface, MotionRuntimeInteface, TimeInterface};
use linear_algebra::{Orientation2, Point2};
use serde::{Deserialize, Serialize};
use types::{
    motion_command::{MotionCommand, OrientationMode},
    motion_runtime::MotionRuntime,
    parameters::    RLWalkingParameters,
    path::traits::{Length, PathProgress},
    step::Step,
};

#[derive(Deserialize, Serialize)]
pub struct BoosterWalking {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    motion_command: Input<MotionCommand, "WorldState", "motion_command">,

    hardware_interface: HardwareInterface,

    parameters: Parameter<RLWalkingParameters, "rl_walking">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterWalking {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }
        let parameters = context.parameters;
        match context.motion_command {
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
                // let deceleration_factor =
                // (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
                // let velocity = forward * *speed * deceleration_factor;

                let velocity = forward * *speed;

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

                move_robot(
                    &context,
                    Step {
                        forward: velocity.x(),
                        left: velocity.y(),
                        turn: angular_velocity,
                    },
                )
            }
            MotionCommand::WalkWithVelocity {
                velocity,
                angular_velocity,
                ..
            } => move_robot(
                &context,
                Step {
                    forward: velocity.x(),
                    left: velocity.y(),
                    turn: *angular_velocity,
                },
            ),
            MotionCommand::Stand { .. } => move_robot(&context, Step::ZERO),
            _ => move_robot(&context, Step::ZERO),
        };

        Ok(MainOutputs {})
    }
}

fn move_robot(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInteface + TimeInterface>,
    step: Step,
) {
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
