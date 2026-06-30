use std::{
    f32::consts::PI,
    time::{Duration, SystemTime},
};

use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::AdditionalOutput;
use hardware::{HighLevelInterface, MotionRuntimeInterface};
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
        mut context: CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.get_motion_runtime_type()? != MotionRuntime::Booster
            || !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }
        let step = step_from_motion_command(context.motion_command, context.parameters);

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

pub fn step_from_motion_command(
    motion_command: &MotionCommand,
    parameters: &RLWalkingParameters,
) -> Step {
    match motion_command {
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

            let (walk_orientation, _tolerance): (Orientation2<Ground>, f32) = match orientation_mode
            {
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

            let angular_velocity = orientation.as_unit_vector().y() * parameters.max_alignment_rate;
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
    }
}

fn move_robot(
    context: &CycleContext<impl HighLevelInterface + MotionRuntimeInterface>,
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

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2};

    use linear_algebra::{Orientation2, point};
    use types::{
        motion_command::{HeadMotion, MotionCommand, OrientationMode},
        parameters::RLWalkingParameters,
        path::direct_path,
    };

    use super::*;

    #[test]
    fn step_aligns_with_path_while_far_from_target() {
        let step = step_from_motion_command(&walk_command_to(2.0), &walking_parameters());

        assert_near(step.forward, 1.0);
        assert_near(step.left, 0.0);
        assert_near(step.turn, 0.0);
    }

    #[test]
    fn step_blends_path_and_target_alignment_in_transition_zone() {
        let step = step_from_motion_command(&walk_command_to(0.55), &walking_parameters());

        assert_near(step.forward, 1.0);
        assert_near(step.left, 0.0);
        assert_near(step.turn, FRAC_1_SQRT_2 * 2.0);
    }

    #[test]
    fn step_aligns_with_target_when_close_enough() {
        let step = step_from_motion_command(&walk_command_to(0.04), &walking_parameters());

        assert_near(step.forward, 0.08);
        assert_near(step.left, 0.0);
        assert_near(step.turn, 2.0);
    }

    fn walking_parameters() -> RLWalkingParameters {
        RLWalkingParameters {
            hybrid_align_distance: 1.0,
            max_alignment_rate: 2.0,
            deceleration_distance: 0.5,
            ..Default::default()
        }
    }

    fn walk_command_to(distance: f32) -> MotionCommand {
        MotionCommand::Walk {
            head: HeadMotion::ZeroAngles,
            path: direct_path(point![0.0, 0.0], point![distance, 0.0]),
            orientation_mode: OrientationMode::AlignWithPath,
            target_orientation: Orientation2::new(FRAC_PI_2),
            distance_to_be_aligned: 0.05,
            speed: 1.0,
        }
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {actual} to be near {expected}",
        );
    }
}
