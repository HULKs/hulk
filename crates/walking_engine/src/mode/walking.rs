use super::{kicking::Kicking, stopping::Stopping, Mode, WalkTransition};
use coordinate_systems::{Ground, Robot, Walk};
use geometry::is_inside_polygon::is_inside_convex_hull;
use linear_algebra::{point, Isometry3, Point3};
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
    feet::Feet, kick_state::KickState, step_plan::StepPlan, step_state::StepState,
    stiffness::Stiffness as _, Context,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct Walking {
    pub step: StepState,
    pub requested_step: Step,
}

impl Walking {
    pub fn new(
        context: &Context,
        requested_step: Step,
        support_side: Side,
        last_requested_step: Step,
    ) -> Self {
        let (backward_acceleration, forward_acceleration) = if last_requested_step.forward > 0.0 {
            (
                -last_requested_step.forward,
                context.parameters.max_forward_acceleration,
            )
        } else if last_requested_step.forward == 0.0 {
            (
                -context.parameters.max_forward_acceleration,
                context.parameters.max_forward_acceleration,
            )
        } else {
            (
                -context.parameters.max_forward_acceleration,
                -last_requested_step.forward,
            )
        };

        let turn_acceleration =
            if last_requested_step.forward.abs() > context.parameters.forward_turn_threshold {
                context.parameters.max_turn_acceleration * context.parameters.forward_turn_reduction
            } else {
                context.parameters.max_turn_acceleration
            };

        let requested_step = Step {
            forward: last_requested_step.forward
                + (requested_step.forward - last_requested_step.forward)
                    .clamp(backward_acceleration, forward_acceleration),
            left: requested_step.left,
            turn: last_requested_step.turn
                + (requested_step.turn - last_requested_step.turn)
                    .clamp(-turn_acceleration, turn_acceleration),
        };
        let plan = StepPlan::new_from_request(context, requested_step, support_side);
        let step = StepState::new(plan);
        Self {
            step,
            requested_step,
        }
    }

    pub fn new_in_flight(self, context: &Context, requested_step: Step) -> Self {
        let support_side = self.step.plan.support_side;
        let start_feet = self.step.plan.start_feet;
        let plan = StepPlan::new_with_start_feet(context, requested_step, support_side, start_feet);
        let step = self.step.update_plan(plan);
        Self {
            step,
            requested_step,
        }
    }
}

impl WalkTransition for Walking {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context)
            || current_step.is_timeouted(context.parameters)
        {
            return Mode::Stopping(Stopping::new(
                context,
                current_step.plan.support_side.opposite(),
            ));
        }

        Mode::Walking(self)
    }

    fn walk(self, context: &Context, requested_step: Step) -> Mode {
        let current_step = self.step;

        if context.parameters.catching_steps.enabled {
            let Some(robot_to_ground) = context.robot_to_ground else {
                return Mode::Stopping(Stopping::new(context, current_step.plan.support_side));
            };

            // let center_of_mass = robot_to_ground * *context.center_of_mass;
            let zero_moment_point: Point3<Ground> = point![
                context.zero_moment_point.x(),
                context.zero_moment_point.y(),
                0.0
            ];
            let robot_to_walk = context.robot_to_walk;
            let ground_to_robot = robot_to_ground.inverse();

            // the blue feet
            let current_feet = Feet::from_joints(
                robot_to_walk,
                &context.last_actuated_joints,
                self.step.plan.support_side,
            );

            if is_outside_support_polygon(
                &self.step.plan,
                zero_moment_point,
                robot_to_walk,
                ground_to_robot,
                current_feet,
            ) {
                let target = robot_to_walk * ground_to_robot * zero_moment_point;
                let support_to_target = target - current_feet.support_sole.position();

                let adjust_distance = support_to_target.x().clamp(-0.1, 0.1);
                let adjusted_adjust_distance = if adjust_distance < 0.0 {
                    adjust_distance + 0.02
                } else {
                    adjust_distance - 0.08
                };

                let request = Step {
                    forward: adjusted_adjust_distance,
                    left: 0.0,
                    turn: 0.0,
                };
                return Mode::Walking(self.new_in_flight(context, request));
            };
        }
        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                self.requested_step,
            ));
        }

        if current_step.is_support_switched(context) {
            return Mode::Walking(Walking::new(
                context,
                requested_step,
                current_step.plan.support_side.opposite(),
                self.requested_step,
            ));
        }

        Mode::Walking(self)
    }

    fn kick(
        self,
        context: &Context,
        variant: KickVariant,
        kicking_side: Side,
        strength: f32,
    ) -> Mode {
        let current_step = self.step;

        if current_step.is_timeouted(context.parameters) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                self.requested_step,
            ));
        }

        if current_step.is_support_switched(context) {
            let next_support_side = current_step.plan.support_side.opposite();
            // TODO: all kicks require a pre-step
            if next_support_side != kicking_side {
                return Mode::Walking(Walking::new(
                    context,
                    Step::ZERO,
                    next_support_side,
                    self.requested_step,
                ));
            }

            return Mode::Kicking(Kicking::new(
                context,
                KickState::new(variant, kicking_side, strength),
                next_support_side,
            ));
        }

        Mode::Walking(self)
    }
}

impl Walking {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(context, true).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_walk,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &Context) {
        self.step.tick(context);
    }
}

fn is_outside_support_polygon(
    plan: &StepPlan,
    zero_moment_point: Point3<Ground>,
    robot_to_walk: Isometry3<Robot, Walk>,
    ground_to_robot: Isometry3<Ground, Robot>,
    current_feet: Feet,
) -> bool {
    #[derive(Copy, Clone, Debug)]
    struct SupportSole;
    let upcoming_walk_to_support_sole = plan
        .end_feet
        .support_sole
        .as_transform::<SupportSole>()
        .inverse();
    // the red swing foot
    let target_swing_sole = current_feet.support_sole.as_transform()
        * upcoming_walk_to_support_sole
        * plan.end_feet.swing_sole;

    let (support_sole_outline, swing_sole_outline, target_swing_sole_outline) =
        if plan.support_side == Side::Left {
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
    let zero_moment_point_in_walk = robot_to_walk * ground_to_robot * zero_moment_point;

    !is_inside_convex_hull(&feet_outlines, &zero_moment_point_in_walk.xy())
}
