use std::time::Duration;

use crate::{
    parameters::{CatchingStepsParameters, Parameters},
    step_plan::StepPlan,
    stiffness::Stiffness as _,
    Context,
};

use super::{
    super::{feet::Feet, step_state::StepState},
    stopping::Stopping,
    walking::Walking,
    Mode, WalkTransition,
};
use coordinate_systems::{Ground, Robot};
use kinematics::forward::{left_sole_to_robot, right_sole_to_robot};
use linear_algebra::{point, Isometry3, Point3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use types::{
    joints::body::BodyJoints, motion_command::KickVariant, motor_commands::MotorCommands,
    step_plan::Step, support_foot::Side,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Catching {
    pub step: StepState,
}

impl Catching {
    pub fn new(
        context: &Context,
        support_side: Side,
        robot_to_ground: Isometry3<Robot, Ground>,
    ) -> Self {
        let parameters = &context.parameters;
        let target_overestimation_factor = context
            .parameters
            .catching_steps
            .target_overestimation_factor;

        let step_duration = parameters.base.step_duration;
        let start_feet =
            Feet::from_joints(context.robot_to_walk, &context.current_joints, support_side);

        let end_feet = catching_end_feet(
            parameters,
            *context.center_of_mass,
            robot_to_ground,
            target_overestimation_factor,
            support_side,
        );
        let max_swing_foot_lift =
            parameters.base.foot_lift_apex + parameters.catching_steps.additional_foot_lift;
        let midpoint = parameters.catching_steps.midpoint;

        let step = StepState {
            plan: StepPlan {
                step_duration,
                start_feet,
                end_feet,
                support_side,
                foot_lift_apex: max_swing_foot_lift,
                midpoint,
            },
            time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
        };
        Self { step }
    }

    fn next_step(self, context: &Context) -> Mode {
        let current_step = self.step;

        let Some(&robot_to_ground) = context.robot_to_ground else {
            return Mode::Stopping(Stopping::new(context, current_step.plan.support_side));
        };

        if is_in_support_polygon(
            &context.parameters.catching_steps,
            &context.current_joints,
            robot_to_ground,
            *context.center_of_mass,
        ) {
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

fn catching_end_feet(
    parameters: &Parameters,
    center_of_mass: Point3<Robot>,
    robot_to_ground: Isometry3<Robot, Ground>,
    target_overestimation_factor: f32,
    support_side: Side,
) -> Feet {
    let max_adjustment = parameters.catching_steps.max_adjustment;
    let target = project_onto_ground(robot_to_ground, center_of_mass);
    Feet::end_from_request(
        parameters,
        Step {
            forward: (target.x() * target_overestimation_factor)
                .clamp(-max_adjustment, max_adjustment),
            left: 0.0,
            turn: 0.0,
        },
        support_side,
    )
}

fn project_onto_ground(
    robot_to_ground: Isometry3<Robot, Ground>,
    target: Point3<Robot>,
) -> Point3<Ground> {
    let target = robot_to_ground * target;
    point![target.x(), target.y(), 0.0]
}

impl WalkTransition for Catching {
    fn stand(self, context: &Context) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }

    fn walk(self, context: &Context, _requested_step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }

    fn kick(self, context: &Context, _variant: KickVariant, _side: Side, _strength: f32) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context);
        }

        Mode::Catching(self)
    }
}

impl Catching {
    pub fn compute_commands(&self, context: &Context) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(context).apply_stiffness(
            context.parameters.stiffnesses.leg_stiffness_walk,
            context.parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &Context) {
        if let Some(&robot_to_ground) = context.robot_to_ground {
            self.step.plan.end_feet = catching_end_feet(
                context.parameters,
                *context.center_of_mass,
                robot_to_ground,
                context
                    .parameters
                    .catching_steps
                    .target_overestimation_factor,
                self.step.plan.support_side,
            );
        }
        self.step.tick(context);
    }
}

pub fn is_in_support_polygon(
    parameters: &CatchingStepsParameters,
    joints: &BodyJoints,
    robot_to_ground: Isometry3<Robot, Ground>,
    target: Point3<Robot>,
) -> bool {
    let left_sole_to_robot = left_sole_to_robot(&joints.left_leg);
    let right_sole_to_robot = right_sole_to_robot(&joints.right_leg);

    let target_on_ground = (robot_to_ground * target).xy();
    let left_toe = robot_to_ground * left_sole_to_robot * point![parameters.toe_offset, 0.0, 0.0];
    let left_heel = robot_to_ground * left_sole_to_robot * point![parameters.heel_offset, 0.0, 0.0];
    let right_toe = robot_to_ground * right_sole_to_robot * point![parameters.toe_offset, 0.0, 0.0];
    let right_heel =
        robot_to_ground * right_sole_to_robot * point![parameters.heel_offset, 0.0, 0.0];

    let forward_balance_limit = left_toe.x().max(right_toe.x());
    let backward_balance_limit = left_heel.x().min(right_heel.x());

    // Warning: For now this doesn't check the support polygon but only the x-axis.
    (backward_balance_limit..=forward_balance_limit).contains(&target_on_ground.x())
}
