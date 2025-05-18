use std::{f32::consts::FRAC_PI_2, time::Duration};

use coordinate_systems::{Ground, LeftSole, RightSole, Robot, Walk};
use geometry::is_inside_polygon::is_inside_convex_hull;
use kinematics::inverse::leg_angles;
use linear_algebra::{point, Isometry3, Orientation3, Point3, Pose3, Rotation3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints, mirror::Mirror},
    robot_dimensions::{
        transform_left_sole_outline, transform_right_sole_outline, RobotDimensions,
    },
    step::Step,
    support_foot::Side,
};

use crate::{
    compensate_stiffness_loss::CompensateStiffnessLossExt,
    parameters::{Parameters, SwingingArmsParameters},
    Context,
};

use super::{
    feet::Feet,
    foot_leveling::{FootLeveling, FootLevelingExt},
    gyro_balancing::{GyroBalancing, GyroBalancingExt},
    step_plan::StepPlan,
};

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct StepState {
    pub plan: StepPlan,
    pub time_since_start: Duration,
    pub gyro_balancing: GyroBalancing,
    pub foot_leveling: FootLeveling,
}

impl StepState {
    pub fn new(plan: StepPlan) -> Self {
        StepState {
            plan,
            time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
        }
    }

    pub fn tick(&mut self, context: &Context) {
        self.time_since_start += context.cycle_time.last_cycle_duration;
        self.gyro_balancing.tick(context);
        self.foot_leveling
            .tick(context, self.normalized_time_since_start());

        if context.parameters.catching_steps.enabled {
            let Some(robot_to_ground) = context.robot_to_ground else {
                return;
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
                self.plan.support_side,
            );

            if is_outside_support_polygon(
                &self.plan,
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

                let support_side = self.plan.support_side;
                let start_feet = self.plan.start_feet;
                let plan =
                    StepPlan::new_with_start_feet(context, request, support_side, start_feet);
                self.plan = plan;
            };
        }
    }

    pub fn is_support_switched(&self, context: &Context) -> bool {
        let pressure_left = context.force_sensitive_resistors.left.sum()
            > context.parameters.sole_pressure_threshold;
        let pressure_right = context.force_sensitive_resistors.right.sum()
            > context.parameters.sole_pressure_threshold;

        let minimal_time = self.time_since_start > context.parameters.min_step_duration;
        let is_support_switched = match self.plan.support_side {
            Side::Left => pressure_right,
            Side::Right => pressure_left,
        };

        minimal_time && is_support_switched
    }

    pub fn is_timeouted(&self, parameters: &Parameters) -> bool {
        self.time_since_start > parameters.max_step_duration
    }

    pub fn compute_joints(&self, context: &Context, feet: Feet) -> BodyJoints {
        let (left_sole, right_sole) = match self.plan.support_side {
            Side::Left => (feet.support_sole, feet.swing_sole),
            Side::Right => (feet.swing_sole, feet.support_sole),
        };
        let walk_to_robot = context.robot_to_walk.inverse();

        let left_foot: Pose3<LeftSole> = Isometry3::from(RobotDimensions::LEFT_ANKLE_TO_LEFT_SOLE)
            .inverse()
            .as_pose();
        let left_sole_to_robot = (walk_to_robot * left_sole).as_transform();
        let right_foot: Pose3<RightSole> =
            Isometry3::from(RobotDimensions::RIGHT_ANKLE_TO_RIGHT_SOLE)
                .inverse()
                .as_pose();
        let right_sole_to_robot = (walk_to_robot * right_sole).as_transform();

        let leg_joints = leg_angles(
            left_sole_to_robot * left_foot,
            right_sole_to_robot * right_foot,
        )
        .balance_using_gyro(&self.gyro_balancing, self.plan.support_side)
        .level_swing_foot(&self.foot_leveling, self.plan.support_side)
        .compensate_stiffness_loss(
            &context.parameters.stiffness_loss_compensation,
            &context.last_actuated_joints.into(),
            &context.measured_joints.into(),
            self.plan.support_side,
        );

        let left_arm = swinging_arm(
            &context.parameters.swinging_arms,
            leg_joints.left_leg,
            right_sole.position().x(),
        );
        let right_arm = swinging_arm(
            &context.parameters.swinging_arms,
            leg_joints.right_leg.mirrored(),
            left_sole.position().x(),
        )
        .mirrored();

        BodyJoints {
            left_arm,
            right_arm,
            left_leg: leg_joints.left_leg,
            right_leg: leg_joints.right_leg,
        }
    }

    fn normalized_time_since_start(&self) -> f32 {
        (self.time_since_start.as_secs_f32() / self.plan.step_duration.as_secs_f32())
            .clamp(0.0, 1.0)
    }

    pub fn compute_feet(&self, context: &Context) -> Feet {
        let parameters = context.parameters;
        let mut support_position = self.support_sole_position(parameters);
        let support_turn = self.support_orientation(parameters);
        let mut swing_position = self.swing_sole_position();
        let swing_turn = self.swing_orientation(parameters);

        let current_feet = Feet::from_joints(
            context.robot_to_walk,
            &context.last_actuated_joints,
            self.plan.support_side,
        );

        let max_speed = context.parameters.max_foot_speed;
        let max_movement = max_speed * context.cycle_time.last_cycle_duration.as_secs_f32();
        let support_sole_movement =
            support_position.xy() - current_feet.support_sole.position().xy();
        let clamped_support_movement = if support_sole_movement.norm() > max_movement {
            support_sole_movement.normalize() * max_movement
        } else {
            support_sole_movement
        };
        support_position = (current_feet.support_sole.position().xy() + clamped_support_movement)
            .extend(support_position.z());

        let swing_sole_movement = swing_position.xy() - current_feet.swing_sole.position().xy();
        let clamped_swing_movement = if swing_sole_movement.norm() > max_movement {
            swing_sole_movement.normalize() * max_movement
        } else {
            swing_sole_movement
        };
        swing_position = (current_feet.swing_sole.position().xy() + clamped_swing_movement)
            .extend(swing_position.z());

        let support_sole = Pose3::from_parts(support_position, support_turn);
        let swing_sole = Pose3::from_parts(swing_position, swing_turn);

        Feet {
            support_sole,
            swing_sole,
        }
    }

    fn support_sole_position(&self, parameters: &Parameters) -> Point3<Walk> {
        let normalized_time = self.normalized_time_since_start();

        let start_offsets = self.plan.start_feet.support_sole.position().xy();
        let end_offsets = self.plan.end_feet.support_sole.position().xy();
        let offsets = start_offsets.lerp(end_offsets, normalized_time);

        let lift = self.support_sole_lift_at(parameters);

        point![offsets.x(), offsets.y(), lift]
    }

    fn support_sole_lift_at(&self, parameters: &Parameters) -> f32 {
        let start_lift = self.plan.start_feet.support_sole.position().z();
        let end_lift = self.plan.end_feet.support_sole.position().z();

        let max_lift_speed = parameters.max_support_foot_lift_speed;
        let max_lift_delta = self.time_since_start.as_secs_f32() * max_lift_speed;

        start_lift + (end_lift - start_lift).clamp(-max_lift_delta, max_lift_delta)
    }

    fn support_orientation(&self, parameters: &Parameters) -> Orientation3<Walk> {
        let normalized_time = self.normalized_time_since_start();
        let start = self.plan.start_feet.support_sole.orientation();
        let target = self.plan.end_feet.support_sole.orientation();

        let max_rotation_delta =
            self.time_since_start.as_secs_f32() * parameters.max_rotation_speed;

        let (roll, pitch, yaw) = start.rotation_to(target).inner.euler_angles();
        let interpolated_roll = roll.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_pitch = pitch.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_yaw = f32::lerp(normalized_time, 0.0, yaw);
        let interpolated =
            Rotation3::from_euler_angles(interpolated_roll, interpolated_pitch, interpolated_yaw);

        interpolated * start
    }

    fn swing_sole_position(&self) -> Point3<Walk> {
        let normalized_time = self.normalized_time_since_start();
        let parabolic_time = parabolic_step(normalized_time);

        let start_offsets = self.plan.start_feet.swing_sole.position().xy();
        let end_offsets = self.plan.end_feet.swing_sole.position().xy();

        let offsets = start_offsets.lerp(end_offsets, parabolic_time);
        let lift = self.swing_sole_lift_at();

        point![offsets.x(), offsets.y(), lift]
    }

    fn swing_sole_lift_at(&self) -> f32 {
        let normalized_time = self.normalized_time_since_start();
        let parabolic_time = parabolic_return(normalized_time, self.plan.midpoint);

        let start_lift = self.plan.start_feet.swing_sole.position().z();
        let end_lift = self.plan.end_feet.swing_sole.position().z();

        let linear_lift = f32::lerp(normalized_time, start_lift, end_lift);
        let parabolic_lift = self.plan.foot_lift_apex * parabolic_time;

        parabolic_lift + linear_lift
    }

    fn swing_orientation(&self, parameters: &Parameters) -> Orientation3<Walk> {
        let normalized_time = self.normalized_time_since_start();
        let start = self.plan.start_feet.swing_sole.orientation();
        let target = self.plan.end_feet.swing_sole.orientation();

        let max_rotation_speed = parameters.max_rotation_speed;
        let max_rotation_delta = self.time_since_start.as_secs_f32() * max_rotation_speed;

        let (roll, pitch, yaw) = start.rotation_to(target).inner.euler_angles();
        let interpolated_roll = roll.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_pitch = pitch.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_yaw = f32::lerp(normalized_time, 0.0, yaw);
        let interpolated =
            Rotation3::from_euler_angles(interpolated_roll, interpolated_pitch, interpolated_yaw);

        interpolated * start
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

fn swinging_arm(
    parameters: &SwingingArmsParameters,
    same_leg: LegJoints,
    opposite_foot_x: f32,
) -> ArmJoints {
    let shoulder_roll = parameters.default_roll + parameters.roll_factor * same_leg.hip_roll;
    let shoulder_pitch = FRAC_PI_2 - opposite_foot_x * parameters.pitch_factor;
    ArmJoints {
        shoulder_pitch,
        shoulder_roll,
        elbow_yaw: 0.0,
        elbow_roll: 0.0,
        wrist_yaw: -FRAC_PI_2,
        hand: 0.0,
    }
}

// visualized in desmos: https://www.desmos.com/calculator/kcr3uxqmyw
fn parabolic_return(x: f32, midpoint: f32) -> f32 {
    if x < midpoint {
        -2.0 / midpoint.powi(3) * x.powi(3) + 3.0 / midpoint.powi(2) * x.powi(2)
    } else {
        -1.0 / (midpoint - 1.0).powi(3)
            * (2.0 * x.powi(3) - 3.0 * (midpoint + 1.0) * x.powi(2) + 6.0 * midpoint * x
                - 3.0 * midpoint
                + 1.0)
    }
}

fn parabolic_step(x: f32) -> f32 {
    if x < 0.5 {
        2.0 * x * x
    } else {
        4.0 * x - 2.0 * x * x - 1.0
    }
}
