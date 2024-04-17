use std::time::Duration;

use crate::motion::walking_engine::{step_plan::StepPlan, stiffness::Stiffness};

use super::{
    super::{feet::Feet, step_state::StepState, CycleContext},
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
    joints::body::BodyJoints,
    motion_command::KickVariant,
    motor_commands::MotorCommands,
    step_plan::Step,
    support_foot::Side,
    walking_engine::{CatchingStepsParameters, WalkingEngineParameters},
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct Catching {
    pub step: StepState,
}

impl Catching {
    pub fn new(
        context: &CycleContext,
        support_side: Side,
        joints: &BodyJoints,
        robot_to_ground: Isometry3<Robot, Ground>,
    ) -> Self {
        let parameters = &context.parameters;
        let target_overestimation_factor = context
            .parameters
            .catching_steps
            .target_overestimation_factor;

        let step_duration = parameters.base.step_duration;
        let start_feet = Feet::from_joints(joints, support_side, parameters);

        let target = project_onto_ground(robot_to_ground, *context.center_of_mass);
        let end_feet = Feet::end_from_request(
            parameters,
            Step {
                forward: target.x() * target_overestimation_factor,
                left: 0.0,
                turn: 0.0,
            },
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

    fn next_step(self, context: &CycleContext, joints: &BodyJoints) -> Mode {
        let current_step = self.step;

        let Some(&robot_to_ground) = context.robot_to_ground else {
            return Mode::Stopping(Stopping::new(
                context,
                current_step.plan.support_side,
                joints,
            ));
        };

        if is_in_support_polygon(
            &context.parameters.catching_steps,
            joints,
            robot_to_ground,
            *context.center_of_mass,
        ) {
            return Mode::Walking(Walking::new(
                context,
                Step::ZERO,
                current_step.plan.support_side.opposite(),
                joints,
                Step::ZERO,
            ));
        }
        Mode::Catching(self)
    }
}

fn project_onto_ground(
    robot_to_ground: Isometry3<Robot, Ground>,
    target: Point3<Robot>,
) -> Point3<Ground> {
    let target = robot_to_ground * target;
    point![target.x(), target.y(), 0.0]
}

impl WalkTransition for Catching {
    fn stand(self, context: &CycleContext, joints: &BodyJoints) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context, joints);
        }

        Mode::Catching(self)
    }

    fn walk(self, context: &CycleContext, joints: &BodyJoints, _requested_step: Step) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context, joints);
        }

        Mode::Catching(self)
    }

    fn kick(
        self,
        context: &CycleContext,
        joints: &BodyJoints,
        _variant: KickVariant,
        _side: Side,
        _strength: f32,
    ) -> Mode {
        let current_step = self.step;
        if current_step.is_support_switched(context) {
            return self.next_step(context, joints);
        }

        Mode::Catching(self)
    }
}

impl Catching {
    pub fn compute_commands(
        &self,
        parameters: &WalkingEngineParameters,
    ) -> MotorCommands<BodyJoints> {
        self.step.compute_joints(parameters).apply_stiffness(
            parameters.stiffnesses.leg_stiffness_walk,
            parameters.stiffnesses.arm_stiffness,
        )
    }

    pub fn tick(&mut self, context: &mut CycleContext, gyro: nalgebra::Vector3<f32>) {
        self.step.tick(context, gyro);
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
