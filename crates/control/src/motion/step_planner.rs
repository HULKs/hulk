use std::time::{Duration, SystemTime};

use color_eyre::{eyre::eyre, Result};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Ground, UpcomingSupport};
use filtering::hysteresis::greater_than_with_absolute_hysteresis;
use framework::{AdditionalOutput, MainOutput};
use geometry::{direction::Rotate90Degrees, look_at::LookAt};
use linear_algebra::{vector, Isometry2, Orientation2, Point2, Pose2};
use step_planning::{
    geometry::{angle::Angle, normalized_step::NormalizedStep, pose::Pose},
    step_plan::StepPlan,
    traits::{EndPoints, Project},
    VARIABLES_PER_STEP,
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
use walking_engine::{anatomic_constraints::AnatomicConstraints, mode::Mode};

#[derive(Deserialize, Serialize)]
pub struct StepPlanner {
    last_step_plan: Option<Vec<f64>>,
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
    step_plan: AdditionalOutput<Vec<Step>, "step_plan">,
    step_plan_greedy: AdditionalOutput<Vec<Step>, "step_plan_greedy">,
    step_plan_gradient: AdditionalOutput<Vec<f32>, "step_plan_gradient">,
    step_plan_cost: AdditionalOutput<f32, "step_plan_cost">,
    current_support_side: AdditionalOutput<Option<Side>, "current_support_side">,
    step_planning_duration: AdditionalOutput<Duration, "step_planning_duration">,
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
                planned_step: Step {
                    forward: 0.0,
                    left: 0.0,
                    turn: 0.0,
                }
                .into(),
            });
        };

        let earlier = SystemTime::now();

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

        let step = if let Some(injected_step) = context.injected_step {
            *injected_step
        } else {
            let step_plan_greedy = step_plan_greedy(
                path,
                &mut context,
                *orientation_mode,
                *target_orientation,
                walk_volume_extents,
            )
            .expect("greedy step planning failed");

            let greedy_step = *step_plan_greedy.first().unwrap();

            context
                .step_plan_greedy
                .fill_if_subscribed(|| step_plan_greedy);

            match context.mode {
                StepPlannerMode::Mpc => self.plan_step(
                    path,
                    &mut context,
                    *orientation_mode,
                    *target_orientation,
                    *distance_to_be_aligned,
                )?,
                StepPlannerMode::Greedy => greedy_step,
            }
        };

        let elapsed = SystemTime::now().duration_since(earlier).unwrap();

        context
            .step_planning_duration
            .fill_if_subscribed(|| elapsed);

        let next_support_side = context
            .walking_engine_mode
            .support_side()
            .unwrap_or(Side::Left)
            .opposite();

        let step = Step {
            forward: step.forward * context.request_scale.forward,
            left: step.left * context.request_scale.left,
            turn: step.turn * context.request_scale.turn,
        };

        let step = clamp_step_size(step, next_support_side, walk_volume_extents);

        Ok(MainOutputs {
            planned_step: step.into(),
        })
    }

    fn plan_step(
        &mut self,
        path: &Path,
        context: &mut CycleContext,
        orientation_mode: OrientationMode,
        target_orientation: Orientation2<Ground>,
        distance_to_be_aligned: f32,
    ) -> Result<Step> {
        let num_variables = context.optimization_parameters.num_steps * VARIABLES_PER_STEP;

        let current_support_side = context.walking_engine_mode.support_side();

        context
            .current_support_side
            .fill_if_subscribed(|| current_support_side);

        match (current_support_side, self.last_support_side) {
            (Some(current_side), Some(last_side)) if current_side != last_side => {
                self.last_step_plan = None;
            }
            _ => {}
        }

        let next_support_side = current_support_side.unwrap_or(Side::Left).opposite();

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
            context
                .direct_step
                .fill_if_subscribed(|| direct_step_to_target);

            self.last_step_plan = None;

            return Ok(direct_step_to_target);
        }

        let variables = if context.optimization_parameters.warm_start {
            self.last_step_plan.get_or_insert(vec![0.0; num_variables])
        } else {
            &mut vec![0.0; num_variables]
        };

        let (gradient, cost) = step_planning_solver::plan_steps(
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            upcoming_support_pose_in_ground(context),
            next_support_side,
            variables.as_mut_slice(),
            context.walk_volume_extents,
            context.optimization_parameters,
        )?;

        let variables_f32: Vec<f32> = variables.iter().map(|&x| x as f32).collect();

        let step_plan: Vec<Step> = StepPlan::from(variables_f32.as_slice())
            .steps()
            .scan(next_support_side, |support_side, step| {
                let result = step.unnormalize(context.walk_volume_extents, *support_side);
                *support_side = support_side.opposite();

                Some(result)
            })
            .collect();

        let next_step = *step_plan.first().unwrap();

        context.step_plan.fill_if_subscribed(|| step_plan);

        context
            .step_plan_gradient
            .fill_if_subscribed(|| gradient.as_slice().to_vec());

        context.step_plan_cost.fill_if_subscribed(|| cost);

        Ok(next_step)
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
    _target_orientation: Orientation2<Ground>,
    walk_volume_extents: &WalkVolumeExtents,
) -> Result<Vec<Step>> {
    let mut pose = context.ground_to_upcoming_support.inverse().as_pose();
    let mut steps = Vec::new();
    let mut support_side = context
        .walking_engine_mode
        .support_side()
        .unwrap_or(Side::Left)
        .opposite();

    for _ in 0..context.optimization_parameters.num_steps {
        let segment = path
            .segments
            .iter()
            .min_by_key(|segment| {
                NotNan::new((segment.project(pose.position()) - pose.position()).norm_squared())
                    .expect("path distance was NaN")
            })
            .ok_or_else(|| eyre!("empty path provided"))?;

        let target_pose = match segment {
            PathSegment::LineSegment(line_segment) => {
                let direction = line_segment.1 - pose.position();
                let rotation = if direction.norm_squared() < f32::EPSILON {
                    Orientation2::identity()
                } else {
                    let normalized_direction = direction.normalize();
                    Orientation2::from_cos_sin_unchecked(
                        normalized_direction.x(),
                        normalized_direction.y(),
                    )
                };
                Pose2::from_parts(line_segment.1, rotation)
            }
            PathSegment::Arc(arc) => {
                // let start_point = arc.start_point();
                let start_point = arc.project(pose.position());
                let direction = (start_point - arc.circle.center).rotate_90_degrees(arc.direction);
                Pose2::from_parts(
                    start_point + direction,
                    Orientation2::from_vector(direction),
                )
            }
        };

        let step_target = pose.as_transform::<Ground>().inverse() * target_pose;

        let step = Step {
            forward: step_target.position().x(),
            left: step_target.position().y(),
            turn: match orientation_mode {
                OrientationMode::Unspecified => step_target.orientation().angle(),
                OrientationMode::AlignWithPath => {
                    pose.position().look_at(&target_pose.position()).angle()
                }
                OrientationMode::LookTowards { direction, .. } => {
                    (pose.orientation().as_transform::<Ground>().inverse() * direction).angle()
                }
                OrientationMode::LookAt { target, .. } => Point2::origin()
                    .look_at(&(pose.as_transform::<Ground>().inverse() * target))
                    .angle(),
            },
        };

        let step = clamp_step_size(step, support_side, walk_volume_extents)
            .clamp_to_anatomic_constraints(support_side, 0.1, 4.0);

        let step_translation =
            Isometry2::<Ground, Ground>::from_parts(vector![step.forward, step.left], 0.0);
        let step_rotation = Isometry2::<Ground, Ground>::from_parts(vector![0.0, 0.0], step.turn);

        pose = pose.as_transform() * step_rotation * step_translation.as_pose();
        support_side = support_side.opposite();

        steps.push(step);
    }

    Ok(steps)
}

fn upcoming_support_pose_in_ground(context: &CycleContext) -> Pose<f32> {
    let pose = context.ground_to_upcoming_support.inverse().as_pose();

    Pose {
        position: pose.position(),
        orientation: Angle(pose.orientation().angle()),
    }
}
