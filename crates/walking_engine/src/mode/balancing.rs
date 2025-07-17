use super::{walking::Walking, Mode, WalkTransition};
use linear_algebra::{vector, Orientation2, Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step::Step, support_foot::Side,
};

use crate::{
    anatomic_constraints::clamp_feet_to_anatomic_constraints, feet::Feet,
    mode::catching::is_outside_support_polygon, step_plan::StepPlan, step_state::StepState,
    stiffness::Stiffness as _, Context,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Balancing {
    pub step: StepState,
}

impl Balancing {
    pub fn new(context: &Context, last_step_state: StepState, support_side: Side) -> Self {
        let Some(robot_to_ground) = context.robot_to_ground else {
            return Self {
                step: last_step_state,
            };
        };

        let robot_to_walk = context.robot_to_walk;
        let ground_to_robot = robot_to_ground.inverse();

        let target = (robot_to_walk * ground_to_robot * context.zero_moment_point.extend(0.0))
            .xy()
            .coords()
            .component_mul(&vector![0.0, -1.0])
            .as_point();

        let clamped_target = target / target.inner.coords.norm()
            * target
                .inner
                .coords
                .norm()
                .min(context.parameters.balancing_steps.max_target_distance);

        let target_projection_into_foot_support = context
            .parameters
            .foot_support
            .project_point_into_rect(clamped_target);
        let displacement =
            Point2::origin() + (clamped_target - target_projection_into_foot_support);

        let desired_end_feet = Feet {
            support_sole: Pose2::from_parts(
                -displacement * 0.5 * context.parameters.balancing_steps.over_estimation_factor,
                Orientation2::default(),
            ),
            swing_sole: Pose2::from_parts(
                displacement * context.parameters.balancing_steps.over_estimation_factor,
                Orientation2::default(),
            ),
        };

        let clamped_feet =
            clamp_feet_to_anatomic_constraints(desired_end_feet, support_side, context.parameters);

        let start_feet = last_step_state.plan.start_feet;
        let plan = StepPlan::new_with_start_and_end_feet(
            context,
            support_side,
            start_feet,
            clamped_feet.at_ground(),
        );

        Self {
            step: StepState {
                plan,
                ..last_step_state
            },
        }
    }
}

impl WalkTransition for Balancing {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        Mode::Balancing(self)
    }

    fn walk(self, context: &Context, _requested_step: Step) -> Mode {
        let current_step = self.step;

        if current_step.is_support_switched(context) {
            let executed_step = self
                .step
                .plan
                .end_feet
                .to_step(context.parameters, self.step.plan.support_side);

            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                self.step.plan.support_side.opposite(),
                executed_step,
            ));
        }

        Mode::Balancing(self)
    }

    fn kick(
        self,
        context: &Context,
        _variant: KickVariant,
        _kicking_side: Side,
        _strength: f32,
    ) -> Mode {
        let current_step = self.step;

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        Mode::Balancing(self)
    }
}

impl Balancing {
    pub fn compute_commands(&mut self, context: &Context) -> MotorCommands<BodyJoints> {
        let feet = self.step.compute_feet(context);
        self.step.compute_joints(context, feet).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_walk,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &Context) {
        self.step.tick(context);
    }
}

pub fn should_balance(context: &Context, end_feet: Feet, support_side: Side) -> bool {
    let balancing_steps = &context.parameters.balancing_steps;
    if !balancing_steps.enabled {
        return false;
    }
    let Some(robot_to_ground) = context.robot_to_ground else {
        return false;
    };

    let ground_to_robot = robot_to_ground.inverse();
    let robot_to_walk = context.robot_to_walk;

    let current_feet =
        Feet::from_joints(robot_to_walk, &context.last_actuated_joints, support_side);

    let zmp = context.zero_moment_point;

    let tuned_zmp = zmp
        .coords()
        .component_mul(&vector![0.0, balancing_steps.zero_moment_point_y_scale])
        .as_point();

    let target = (robot_to_walk * ground_to_robot * tuned_zmp.extend(0.0)).xy();

    is_outside_support_polygon(end_feet, support_side, target, current_feet)
}
