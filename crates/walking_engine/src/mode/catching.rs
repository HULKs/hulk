use super::{walking::Walking, Mode, WalkTransition};
use coordinate_systems::Walk;
use geometry::{is_inside_polygon::is_inside_convex_hull, rectangle::Rectangle};
use linear_algebra::{point, Orientation2, Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use types::{
    joints::body::BodyJoints,
    motion_command::KickVariant,
    motor_commands::MotorCommands,
    robot_dimensions::{transform_left_sole_outline, transform_right_sole_outline},
    step::Step,
    support_foot::Side,
};

use crate::{
    anatomic_constraints::clamp_feet_to_anatomic_constraints, feet::Feet, step_plan::StepPlan,
    step_state::StepState, stiffness::Stiffness as _, Context,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Catching {
    pub step: StepState,
}

impl Catching {
    pub fn new(context: &Context, last_step_state: StepState) -> Self {
        let Some(robot_to_ground) = context.robot_to_ground else {
            todo!();
        };

        let robot_to_walk = context.robot_to_walk;
        let ground_to_robot = robot_to_ground.inverse();

        let target = (robot_to_walk * ground_to_robot * context.zero_moment_point.extend(0.0)).xy();

        let foot_support = Rectangle {
            min: point![-0.02, -0.02],
            max: point![0.08, 0.02],
        };
        let target_projection_into_foot_support = foot_support.project_point_into_rect(target);
        let displacement = Point2::origin() + (target - target_projection_into_foot_support);

        let desired_end_feet = Feet {
            support_sole: Pose2::from_parts(-displacement / 2., Orientation2::default()),
            swing_sole: Pose2::from_parts(displacement, Orientation2::default()),
        };

        let support_side = last_step_state.plan.support_side;
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

impl WalkTransition for Catching {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;

        if should_catch(
            context,
            current_step.plan.end_feet,
            current_step.plan.support_side,
        ) {
            return Mode::Catching(Catching::new(context, self.step));
        }

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        Mode::Catching(self)
    }

    fn walk(self, context: &Context, _requested_step: Step) -> Mode {
        let current_step = self.step;

        if should_catch(
            context,
            current_step.plan.end_feet,
            current_step.plan.support_side,
        ) {
            return Mode::Catching(Catching::new(context, self.step));
        }

        if current_step.is_support_switched(context) {
            let executed_step = self
                .step
                .plan
                .end_feet
                .to_step(context.parameters, self.step.plan.support_side);

            return Mode::Walking(Walking::new(
                context,
                Step {
                    forward: executed_step.forward / 2.,
                    left: executed_step.left / 2.,
                    turn: 0.0,
                },
                current_step.plan.support_side.opposite(),
                executed_step,
            ));
        }

        Mode::Catching(self)
    }

    fn kick(
        self,
        context: &Context,
        _variant: KickVariant,
        _kicking_side: Side,
        _strength: f32,
    ) -> Mode {
        let current_step = self.step;

        if should_catch(
            context,
            current_step.plan.end_feet,
            current_step.plan.support_side,
        ) {
            return Mode::Catching(Catching::new(context, self.step));
        }

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                Step::ZERO,
            ));
        }

        Mode::Catching(self)
    }
}

impl Catching {
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

pub fn should_catch(context: &Context, end_feet: Feet, support_side: Side) -> bool {
    if !context.parameters.catching_steps.enabled {
        return false;
    }
    let Some(robot_to_ground) = context.robot_to_ground else {
        return false;
    };

    let ground_to_robot = robot_to_ground.inverse();
    let robot_to_walk = context.robot_to_walk;

    let current_feet =
        Feet::from_joints(robot_to_walk, &context.last_actuated_joints, support_side);

    let target = (robot_to_walk * ground_to_robot * context.zero_moment_point.extend(0.0)).xy();

    is_outside_support_polygon(end_feet, support_side, target, current_feet)
}

fn is_outside_support_polygon(
    end_feet: Feet,
    support_side: Side,
    target: Point2<Walk>,
    current_feet: Feet,
) -> bool {
    // the red swing foot
    let target_swing_sole = end_feet.swing_sole;

    let (support_sole_outline, swing_sole_outline, target_swing_sole_outline) =
        if support_side == Side::Left {
            (
                transform_left_sole_outline(current_feet.support_sole.as_transform()),
                transform_right_sole_outline(current_feet.swing_sole.as_transform()),
                transform_right_sole_outline(target_swing_sole.as_transform()),
            )
        } else {
            (
                transform_right_sole_outline(current_feet.support_sole.as_transform()),
                transform_left_sole_outline(current_feet.swing_sole.as_transform()),
                transform_left_sole_outline(target_swing_sole.as_transform()),
            )
        };

    let feet_outlines = [
        swing_sole_outline,
        support_sole_outline,
        target_swing_sole_outline,
    ]
    .concat();

    !is_inside_convex_hull(&feet_outlines, &target)
}
