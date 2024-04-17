use std::{f32::consts::FRAC_PI_2, time::Duration};

use coordinate_systems::{LeftSole, RightSole, Walk};
use kinematics::inverse::leg_angles;
use linear_algebra::{point, Isometry3, Orientation3, Point3, Pose3, Rotation3};
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;
use splines::Interpolate;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints, mirror::Mirror},
    robot_dimensions::RobotDimensions,
    support_foot::Side,
};

use crate::{
    parameters::{Parameters, SwingingArmsParameters},
    Context,
};

use super::{
    feet::{robot_to_walk, Feet},
    foot_leveling::{FootLeveling, FootLevelingExt},
    gyro_balancing::{GyroBalancing, GyroBalancingExt},
    step_plan::StepPlan,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
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
    }

    pub fn is_support_switched(&self, context: &Context) -> bool {
        let pressure_left = context.sensor_data.force_sensitive_resistors.left.sum()
            > context.parameters.sole_pressure_threshold;
        let pressure_right = context.sensor_data.force_sensitive_resistors.right.sum()
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

    pub fn compute_joints(&self, parameters: &Parameters) -> BodyJoints {
        let feet = self.compute_feet(parameters);

        let (left_sole, right_sole) = match self.plan.support_side {
            Side::Left => (feet.support_sole, feet.swing_sole),
            Side::Right => (feet.swing_sole, feet.support_sole),
        };
        let walk_to_robot = robot_to_walk(parameters).inverse();

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
        .level_swing_foot(&self.foot_leveling, self.plan.support_side);

        let left_arm = swinging_arm(
            &parameters.swinging_arms,
            leg_joints.left_leg,
            right_sole.position().x(),
        );
        let right_arm = swinging_arm(
            &parameters.swinging_arms,
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

    fn compute_feet(&self, parameters: &Parameters) -> Feet {
        let support_sole = self.support_sole_position(parameters);
        let support_turn = self.support_orientation(parameters);
        let swing_sole = self.swing_sole_position();
        let swing_turn = self.swing_orientation(parameters);

        let support_sole = Pose3::from_parts(support_sole, support_turn);
        let swing_sole = Pose3::from_parts(swing_sole, swing_turn);

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
