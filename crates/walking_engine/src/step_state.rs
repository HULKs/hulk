use std::{f32::consts::FRAC_PI_2, time::Duration};

use coordinate_systems::{LeftSole, RightSole, Walk};
use kinematics::inverse::leg_angles;
use linear_algebra::{point, Isometry3, Orientation3, Point3, Pose3, Rotation3};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use types::{
    joints::{arm::ArmJoints, body::BodyJoints, leg::LegJoints, mirror::Mirror},
    robot_dimensions::RobotDimensions,
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
    pub xy_time_since_start: Duration,
    pub gyro_balancing: GyroBalancing,
    pub foot_leveling: FootLeveling,
    pub last_engine_feet: Feet,
}

impl StepState {
    pub fn new(plan: StepPlan) -> Self {
        StepState {
            plan,
            time_since_start: Duration::ZERO,
            xy_time_since_start: Duration::ZERO,
            gyro_balancing: Default::default(),
            foot_leveling: Default::default(),
            last_engine_feet: plan.start_feet,
        }
    }

    pub fn tick(&mut self, context: &Context) {
        let parameters = &context.parameters.dynamic_interpolation_speed;
        let default_torso_rotation = context.robot_to_walk.rotation();
        let current_orientation = context.robot_orientation;

        let leveling_error = current_orientation.inner * default_torso_rotation.inner.inverse();
        let (_, pitch_angle, _) = leveling_error.euler_angles();

        let walk_speed_adjustment = if pitch_angle < parameters.active_range.end {
            1.0 - parameters.max_reduction
                * ((pitch_angle - parameters.active_range.end) / parameters.active_range.start)
                    .clamp(0.0, 1.0)
        } else {
            1.0
        };

        self.time_since_start += context
            .cycle_time
            .last_cycle_duration
            .mul_f32(walk_speed_adjustment);
        self.gyro_balancing.tick(context);
        self.foot_leveling
            .tick(context, self.normalized_time_since_start());

        let small_weight_on_sensors = match self.plan.support_side {
            Side::Left => {
                context.force_sensitive_resistors.right.sum() > parameters.xy_offset_stop_weight
            }
            Side::Right => {
                context.force_sensitive_resistors.left.sum() > parameters.xy_offset_stop_weight
            }
        };

        let xy_slip_stop = if small_weight_on_sensors
            && !self.is_support_switched(context)
            && self.normalized_xy_time_since_start() > 0.3
        {
            1.0 - parameters.slip_reduction
        } else {
            1.0
        };

        self.xy_time_since_start = self.time_since_start.mul_f32(xy_slip_stop);
    }

    pub fn is_support_switched(&self, context: &Context) -> bool {
        let sum_left = context.force_sensitive_resistors.left.sum();
        let sum_right = context.force_sensitive_resistors.right.sum();

        let pressure_left = sum_left > context.parameters.sole_pressure_threshold;
        let pressure_right = sum_right > context.parameters.sole_pressure_threshold;

        let minimal_time = self.time_since_start > context.parameters.min_step_duration;
        let is_support_switched = match self.plan.support_side {
            Side::Left => {
                pressure_right
                    || (sum_right > sum_left && sum_right > context.parameters.min_sole_pressure)
            }
            Side::Right => {
                pressure_left
                    || (sum_left > sum_right && sum_left > context.parameters.min_sole_pressure)
            }
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

    fn normalized_xy_time_since_start(&self) -> f32 {
        (self.xy_time_since_start.as_secs_f32() / self.plan.step_duration.as_secs_f32())
            .clamp(0.0, 1.0)
    }

    pub fn compute_feet(&mut self, context: &Context) -> Feet {
        let parameters = &context.parameters;
        let dt = context.cycle_time.last_cycle_duration.as_secs_f32();
        let max_movement = parameters.max_foot_speed * dt;

        let support_position = self.support_sole_position(parameters);
        let support_turn = self.support_orientation(parameters);
        let swing_position = self.swing_sole_position();
        let swing_turn = self.swing_orientation(parameters);

        let limited_support_position = clamp_xy_movement(
            self.last_engine_feet.support_sole.position(),
            support_position,
            max_movement,
        );
        let limited_swing_position = clamp_xy_movement(
            self.last_engine_feet.swing_sole.position(),
            swing_position,
            max_movement,
        );

        let support_sole = Pose3::from_parts(limited_support_position, support_turn);
        let swing_sole = Pose3::from_parts(limited_swing_position, swing_turn);

        let feet = Feet {
            support_sole,
            swing_sole,
        };
        self.last_engine_feet = feet;
        feet
    }

    fn support_sole_position(&self, parameters: &Parameters) -> Point3<Walk> {
        let normalized_xy_time = self.normalized_xy_time_since_start();

        let start_offsets = self.plan.start_feet.support_sole.position().xy();
        let end_offsets = self.plan.end_feet.support_sole.position().xy();
        let offsets = start_offsets.lerp(end_offsets, normalized_xy_time);

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

fn clamp_xy_movement<Frame>(
    from: Point3<Frame>,
    to: Point3<Frame>,
    max_movement: f32,
) -> Point3<Frame> {
    let delta_xy = to.xy() - from.xy();
    let clamped_xy = delta_xy.cap_magnitude(max_movement);
    (from.xy() + clamped_xy).extend(to.z())
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
