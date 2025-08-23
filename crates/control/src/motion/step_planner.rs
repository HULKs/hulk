use std::{array, f32::consts::PI};

use color_eyre::Result;
use itertools::Itertools;
use nalgebra::SVector;
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, UpcomingSupport};
use filtering::hysteresis::greater_than_with_absolute_hysteresis;
use framework::{AdditionalOutput, MainOutput};
use geometry::{direction::Rotate90Degrees, look_at::LookAt};
use linear_algebra::{vector, Isometry2, Orientation2, Point2, Pose2};
use step_planning::{
    geometry::{normalized_step::NormalizedStep, orientation::Orientation, pose::Pose},
    step_plan::StepPlan,
    traits::{EndPoints, Project},
    NUM_STEPS, NUM_VARIABLES,
};
use types::{
    motion_command::{MotionCommand, OrientationMode, WalkSpeed},
    parameters::{StepPlannerMode, StepPlanningOptimizationParameters},
    planned_path::{Path, PathSegment},
    sensor_data::SensorData,
    step::Step,
    support_foot::Side,
    walk_volume_extents::WalkVolumeExtents,
};
use walking_engine::mode::Mode;

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_step_plan: Option<[f64; NUM_VARIABLES]>,
    last_support_side: Option<Side>,
    leg_joints_hot: bool,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motion_command: Input<MotionCommand, "motion_command">,
    sensor_data: Input<SensorData, "sensor_data">,

    injected_step: Parameter<Option<Step>, "step_planner.injected_step?">,
    walk_volume_delta_slow: Parameter<WalkVolumeExtents, "step_planner.walk_volume_delta_slow">,
    walk_volume_delta_fast: Parameter<WalkVolumeExtents, "step_planner.walk_volume_delta_fast">,
    request_scale: Parameter<Step, "step_planner.request_scale">,
    optimization_parameters:
        Parameter<StepPlanningOptimizationParameters, "step_planner.optimization_parameters">,
    walk_volume_extents: Parameter<WalkVolumeExtents, "step_planner.walk_volume_extents">,
    mode: Parameter<StepPlannerMode, "step_planner.mode">,

    ground_to_upcoming_support:
        CyclerState<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    walking_engine_mode: CyclerState<Mode, "walking_engine_mode">,

    ground_to_upcoming_support_out:
        AdditionalOutput<Isometry2<Ground, UpcomingSupport>, "ground_to_upcoming_support">,
    direct_step: AdditionalOutput<Step, "direct_step">,
    step_plan: AdditionalOutput<[Step; step_planning::NUM_STEPS], "step_plan">,
    step_plan_greedy: AdditionalOutput<[Step; step_planning::NUM_STEPS], "step_plan_greedy">,
    step_plan_gradient:
        AdditionalOutput<SVector<f32, { step_planning::NUM_VARIABLES }>, "step_plan_gradient">,
    step_plan_cost: AdditionalOutput<f32, "step_plan_cost">,
    next_support_side: AdditionalOutput<Side, "next_support_side">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub planned_step: MainOutput<Step>,
}

impl StepPlanner {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_step_plan: None,
            last_support_side: None,
            leg_joints_hot: false,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        // TODO(rmburg): Reimplement initial side bonus if needed
        context
            .ground_to_upcoming_support_out
            .fill_if_subscribed(|| *context.ground_to_upcoming_support);

        let MotionCommand::Walk {
            path,
            orientation_mode,
            speed,
            target_orientation,
            distance_to_be_aligned,
            ..
        } = context.motion_command
        else {
            self.last_step_plan = None;
            self.last_support_side = None;
            return Ok(MainOutputs {
                planned_step: Step::ZERO.into(),
            });
        };

        let highest_temperature = context
            .sensor_data
            .temperature_sensors
            .left_leg
            .into_iter()
            .chain(context.sensor_data.temperature_sensors.right_leg)
            .max_by(f32::total_cmp)
            .expect("temperatures to be not empty.");

        self.leg_joints_hot = greater_than_with_absolute_hysteresis(
            self.leg_joints_hot,
            highest_temperature,
            70.0..=75.0,
        );
        // at 76°C stiffness gets automatically reduced by the motors - this stops if temperature is below 70°C again

        let walk_volume_extents = match (speed, self.leg_joints_hot) {
            (WalkSpeed::Fast, false) => {
                &(context.walk_volume_extents + context.walk_volume_delta_fast)
            }
            (WalkSpeed::Slow, _) => &(context.walk_volume_extents + context.walk_volume_delta_slow),
            _ => context.walk_volume_extents,
        };

        let current_support_side = context.walking_engine_mode.support_side();
        let next_support_side = current_support_side.unwrap_or(Side::Left).opposite();

        context
            .next_support_side
            .fill_if_subscribed(|| next_support_side);

        let step = if let Some(injected_step) = context.injected_step {
            *injected_step
        } else if let Some(direct_step) =
            self.try_direct_step(&mut context, path, *target_orientation, next_support_side)
        {
            context.direct_step.fill_if_subscribed(|| direct_step);

            self.last_step_plan = None;

            direct_step
        } else {
            let step_plan_greedy = step_plan_greedy(
                path,
                &mut context,
                *orientation_mode,
                *target_orientation,
                next_support_side,
                *distance_to_be_aligned,
                walk_volume_extents,
            )
            .expect("greedy step planning failed");

            let greedy_step = *step_plan_greedy.first().unwrap();

            context
                .step_plan_greedy
                .fill_if_subscribed(|| step_plan_greedy);

            match context.mode {
                StepPlannerMode::Mpc => self.plan_step_with_mpc(
                    path,
                    &mut context,
                    *orientation_mode,
                    *target_orientation,
                    next_support_side,
                    *distance_to_be_aligned,
                )?,
                StepPlannerMode::Greedy => greedy_step,
            }
        };

        let step = Step {
            forward: step.forward * context.request_scale.forward,
            left: step.left * context.request_scale.left,
            turn: step.turn * context.request_scale.turn,
        };

        Ok(MainOutputs {
            planned_step: step.into(),
        })
    }

    fn plan_step_with_mpc(
        &mut self,
        path: &Path,
        context: &mut CycleContext,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        next_support_side: Side,
        distance_to_be_aligned: f32,
    ) -> Result<Step> {
        let current_support_side = context.walking_engine_mode.support_side();

        match (current_support_side, self.last_support_side) {
            (Some(current_side), Some(last_side)) if current_side != last_side => {
                self.last_step_plan = None;
            }
            _ => {}
        }

        let variables = if context.optimization_parameters.warm_start {
            self.last_step_plan.get_or_insert([0.0; NUM_VARIABLES])
        } else {
            &mut [0.0; NUM_VARIABLES]
        };

        let (gradient, cost) = step_planning_solver::plan_steps(
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            upcoming_support_pose_in_ground(context),
            next_support_side,
            variables,
            context.walk_volume_extents,
            context.optimization_parameters,
        )?;

        let variables_f32: [f32; NUM_VARIABLES] = array::from_fn(|i| variables[i] as f32);

        let step_plan: [Step; NUM_STEPS] = StepPlan::from(variables_f32.as_slice())
            .steps()
            .scan(next_support_side, |support_side, step| {
                let result = step.unnormalize(context.walk_volume_extents, *support_side);
                *support_side = support_side.opposite();

                Some(result)
            })
            .collect_array()
            .expect("failed to collect steps into fixed-size array");

        let next_step = *step_plan.first().expect("step plan was empty");

        context.step_plan.fill_if_subscribed(|| step_plan);
        context.step_plan_gradient.fill_if_subscribed(|| gradient);
        context.step_plan_cost.fill_if_subscribed(|| cost);

        Ok(next_step)
    }

    fn try_direct_step(
        &mut self,
        context: &mut CycleContext<'_>,
        path: &Path,
        target_orientation: Orientation2<Ground>,
        next_support_side: Side,
    ) -> Option<Step> {
        let target_point = path.end_point();
        let target_pose = Pose2::from_parts(target_point, target_orientation);
        let target_pose_in_upcoming_support = *context.ground_to_upcoming_support * target_pose;
        let direct_step_to_target = Step::from_pose(target_pose_in_upcoming_support);
        let normalized_direct_step_to_target = NormalizedStep::from_step(
            direct_step_to_target,
            context.walk_volume_extents,
            next_support_side,
        );

        if normalized_direct_step_to_target.is_inside_walk_volume() {
            Some(direct_step_to_target)
        } else {
            None
        }
    }
}

fn clamp_step_size(
    step: Step,
    support_side: Side,
    walk_volume_extents: &WalkVolumeExtents,
) -> Step {
    NormalizedStep::from_step(step, walk_volume_extents, support_side)
        .clamp_to_walk_volume()
        .unnormalize(walk_volume_extents, support_side)
}

fn step_plan_greedy(
    path: &Path,
    context: &mut CycleContext,
    orientation_mode: OrientationMode,
    target_orientation: Orientation2<Ground>,
    next_support_side: Side,
    distance_to_be_aligned: f32,
    walk_volume_extents: &WalkVolumeExtents,
) -> Result<[Step; NUM_STEPS]> {
    let initial_pose = context.ground_to_upcoming_support.inverse().as_pose();

    let steps = (0..NUM_STEPS)
        .scan(
            (initial_pose, next_support_side),
            |(pose, support_side), _i| {
                let segment = path
                    .segments
                    .iter()
                    .min_by_key(|segment| {
                        NotNan::new(
                            (segment.project(pose.position()) - pose.position()).norm_squared(),
                        )
                        .expect("path distance was NaN")
                    })
                    .expect("path was empty");

                let target_pose = match segment {
                    PathSegment::LineSegment(line_segment) => {
                        let direction = line_segment.1 - pose.position();
                        let rotation = if direction.norm_squared() < f32::EPSILON {
                            Orientation2::identity()
                        } else {
                            Orientation2::from_vector(direction)
                        };
                        Pose2::from_parts(line_segment.1, rotation)
                    }
                    PathSegment::Arc(arc) => {
                        let start_point = arc.project(pose.position());
                        let direction =
                            (start_point - arc.circle.center).rotate_90_degrees(arc.direction);
                        Pose2::from_parts(
                            start_point + direction,
                            Orientation2::from_vector(direction),
                        )
                    }
                };

                struct CurrentPose;
                let ground_to_current_pose = pose.as_transform::<CurrentPose>().inverse();

                let step_target = ground_to_current_pose * target_pose;

                let step = Step {
                    forward: step_target.position().x(),
                    left: step_target.position().y(),
                    turn: match orientation_mode {
                        OrientationMode::Unspecified | OrientationMode::AlignWithPath => {
                            let to_step_target = target_pose.position() - pose.position();
                            let step_target_orientation = hybrid_alignment(
                                target_orientation,
                                Orientation2::from_vector(to_step_target),
                                to_step_target.norm(),
                                context.optimization_parameters.hybrid_align_distance,
                                distance_to_be_aligned,
                            );

                            step_target_orientation.angle()
                        }
                        OrientationMode::LookTowards { direction, .. } => {
                            (pose.orientation().as_transform::<Ground>().inverse() * direction)
                                .angle()
                        }
                        OrientationMode::LookAt { target, .. } => Point2::origin()
                            .look_at(&(pose.as_transform::<Ground>().inverse() * target))
                            .angle(),
                    },
                };

                let step = clamp_step_size(step, *support_side, walk_volume_extents);
                let step = clamp_inside_movement(step, *support_side);

                let step_translation =
                    Isometry2::<Ground, Ground>::from_parts(vector![step.forward, step.left], 0.0);
                let step_rotation =
                    Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], step.turn);

                *pose = pose.as_transform() * step_rotation * step_translation.as_pose();
                *support_side = support_side.opposite();

                Some(step)
            },
        )
        .collect_array()
        .unwrap();

    Ok(steps)
}

fn clamp_inside_movement(mut step: Step, support_side: Side) -> Step {
    match support_side {
        Side::Left => step.left = step.left.min(0.0),
        Side::Right => step.left = step.left.max(0.0),
    }

    step
}

fn upcoming_support_pose_in_ground(context: &CycleContext) -> Pose<f32> {
    let pose = context.ground_to_upcoming_support.inverse().as_pose();

    Pose {
        position: pose.position(),
        orientation: Orientation(pose.orientation().angle()),
    }
}

pub fn hybrid_alignment(
    target_orientation: Orientation2<Ground>,
    forward_orientation: Orientation2<Ground>,
    distance_to_target: f32,
    hybrid_align_distance: f32,
    distance_to_be_aligned: f32,
) -> Orientation2<Ground> {
    if distance_to_target > distance_to_be_aligned + hybrid_align_distance {
        return forward_orientation;
    }

    let angle_limit = ((distance_to_target - distance_to_be_aligned) / hybrid_align_distance)
        .clamp(0.0, 1.0)
        * PI;

    clamp_around(forward_orientation, target_orientation, angle_limit)
}

pub fn clamp_around(
    input: Orientation2<Ground>,
    center: Orientation2<Ground>,
    angle_limit: f32,
) -> Orientation2<Ground> {
    let center_to_input = center.rotation_to(input);
    let clamped = center_to_input.clamp_angle::<Ground>(-angle_limit, angle_limit);

    clamped * center
}

#[cfg(test)]
mod test {
    use super::*;

    use std::f32::consts::{FRAC_PI_2, PI};

    use approx::assert_relative_eq;
    use num_traits::Zero;

    #[test]
    fn clamp_noop_when_less_than_limit_around_center() {
        let testcases = [
            (0.0, 0.0),
            (0.0, PI),
            (1.0, FRAC_PI_2),
            (-1.0, FRAC_PI_2),
            (FRAC_PI_2, FRAC_PI_2),
            (-FRAC_PI_2, FRAC_PI_2),
        ];

        for (input, angle_limit) in testcases {
            let input = Orientation2::new(input);
            let center = Orientation2::new(0.0);
            assert_relative_eq!(clamp_around(input, center, angle_limit), input);
        }
    }

    #[test]
    fn clamp_clamps_to_limit_around_center() {
        let testcases = [
            (0.0, 0.0),
            (PI, PI),
            (2.0, FRAC_PI_2),
            (-2.0, FRAC_PI_2),
            (FRAC_PI_2, FRAC_PI_2),
            (-FRAC_PI_2, FRAC_PI_2),
            (PI - f32::EPSILON, FRAC_PI_2),
            (-PI + f32::EPSILON, FRAC_PI_2),
        ];

        for (input, angle_limit) in testcases {
            let input = Orientation2::new(input);
            let center = Orientation2::new(0.0);

            let output = clamp_around(input, center, angle_limit);

            assert_relative_eq!(output.angle().abs(), angle_limit);
            assert_eq!(output.angle().signum(), input.angle().signum())
        }
    }

    #[test]
    fn clamped_always_closer_than_limit() {
        let angles = [
            0.0,
            PI - 0.01,
            -PI + 0.01,
            FRAC_PI_2,
            -FRAC_PI_2,
            1.0,
            -1.0,
            2.0,
            -2.0,
        ];

        for input in angles {
            for center in angles {
                for angle_limit in angles {
                    let angle_limit = angle_limit.abs();
                    let input = Orientation2::new(input);
                    let center = Orientation2::new(center);

                    let output = clamp_around(input, center, angle_limit);

                    let relative_output = center.rotation_to(output);
                    let relative_input = center.rotation_to(input);
                    assert!(relative_output.angle().abs() <= angle_limit);
                    if !relative_output.angle().is_zero() {
                        assert_eq!(
                            relative_output.angle().signum(),
                            relative_input.angle().signum()
                        )
                    }
                }
            }
        }
    }
}
