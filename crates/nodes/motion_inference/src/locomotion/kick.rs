use std::f32::consts::{PI, TAU};

use ::kinematics::joints::Joints;
use coordinate_systems::{Ground, Robot};
use linear_algebra::{Point2, Vector3};
use nalgebra::{UnitComplex, UnitQuaternion};

use super::{KickRequest, Locomotion, leg};
use crate::{
    config::{KickParameters, LEGS, Policy, clip_measurement},
    observation::SensorFrame,
};

/// Strong and soft kick share this tensor contract, but not their flag/speed rules.
pub struct Observation {
    pub gravity: Vector3<Robot>,
    pub angular_velocity: Vector3<Robot>,
    pub ball_position: Point2<Ground>,
    pub direction: f32,
    pub phase: [f32; 2],
    pub position_offsets: Joints<f32>,
    pub joint_velocity: Joints<f32>,
    pub previous_target_offsets: Joints<f32>,
    pub frequency_offset: f32,
    pub strong: bool,
    pub quick: bool,
    pub ball_velocity: linear_algebra::Vector2<Ground>,
    pub target_speed: f32,
    pub previous_ball_position: Point2<Ground>,
    pub joint_velocity_scale: f32,
    pub ball_position_scale: f32,
    pub ball_velocity_scale: f32,
    pub target_speed_scale: f32,
}

impl Observation {
    pub fn new(
        state: &Locomotion,
        sensor: &SensorFrame,
        joint_velocity: &Joints<f32>,
        standing: bool,
        request: KickRequest,
        soft: bool,
        (ball_position, previous_ball_position): (Point2<Ground>, Point2<Ground>),
    ) -> Self {
        let strong = request.strong && !soft;
        let parameters = &state.parameters;
        let [minimum_speed, maximum_speed] = if soft {
            parameters.kick.soft_speed_limits
        } else {
            parameters.kick.speed_limits
        };
        let offset = if soft { Policy::SoftKick } else { Policy::Kick }.offset(parameters);
        Self {
            joint_velocity_scale: parameters.observation.joint_velocity_scale,
            ball_position_scale: parameters.kick.ball_position_scale,
            ball_velocity_scale: parameters.kick.ball_velocity_scale,
            target_speed_scale: parameters.kick.target_speed_scale,
            gravity: Vector3::wrap(sensor.gravity().into()),
            angular_velocity: sensor.gyro,
            ball_position,
            direction: request.direction,
            phase: state.phase_encoding(standing),
            position_offsets: clip_measurement(sensor.position, parameters.joint_limits) - offset,
            joint_velocity: *joint_velocity,
            previous_target_offsets: state.previous_target - offset,
            frequency_offset: state.frequency_offset,
            strong,
            quick: request.quick && !soft,
            ball_velocity: request
                .ball_velocity
                .cap_magnitude(parameters.kick.ball_velocity_limit),
            target_speed: if strong {
                maximum_speed
            } else {
                request.target_speed.clamp(minimum_speed, maximum_speed)
            },
            previous_ball_position,
        }
    }

    pub fn to_tensor(&self) -> [f32; 59] {
        let mut input = [0.0; 59];
        input[..3].copy_from_slice(self.gravity.inner.as_slice());
        input[3..6].copy_from_slice(self.angular_velocity.inner.as_slice());
        input[6..9].copy_from_slice(&[
            self.ball_position.x() * self.ball_position_scale,
            self.ball_position.y() * self.ball_position_scale,
            normalize_angle(self.direction) / PI,
        ]);
        input[9..11].copy_from_slice(&self.phase);
        input[11..23].copy_from_slice(&LEGS.map(|joint| self.position_offsets[joint]));
        input[23..35].copy_from_slice(
            &LEGS.map(|joint| self.joint_velocity[joint] * self.joint_velocity_scale),
        );
        input[35..47].copy_from_slice(&LEGS.map(|joint| self.previous_target_offsets[joint]));
        input[47] = self.frequency_offset;
        input[48..51].copy_from_slice(&[f32::from(self.strong), 0.0, f32::from(self.quick)]);
        input[51..54].copy_from_slice(&[
            self.ball_velocity.x() * self.ball_velocity_scale,
            self.ball_velocity.y() * self.ball_velocity_scale,
            0.0,
        ]);
        input[54..56].copy_from_slice(&[self.direction.sin(), self.direction.cos()]);
        input[56] = self.target_speed * self.target_speed_scale;
        input[57..59].copy_from_slice(&[
            self.previous_ball_position.x() * self.ball_position_scale,
            self.previous_ball_position.y() * self.ball_position_scale,
        ]);
        input
    }
}

fn normalize_angle(angle: f32) -> f32 {
    // The trained feature uses [-PI, PI); atan2-based orientations can return +PI.
    (angle + PI).rem_euclid(TAU) - PI
}

fn ramp(value: f32, minimum: f32, maximum: f32) -> f32 {
    ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0)
}

pub(super) fn shifted_ball(
    sensor: &SensorFrame,
    kick: KickRequest,
    parameters: &KickParameters,
) -> Point2<Ground> {
    let to_kick_direction = UnitComplex::new(-kick.direction);
    let (roll, pitch, _) = sensor.rotation().euler_angles();
    let level = UnitQuaternion::from_euler_angles(roll, pitch, 0.0);
    let left = to_kick_direction * (level * leg(&sensor.position, true).0.inner.coords).xy();
    let right = to_kick_direction * (level * leg(&sensor.position, false).0.inner.coords).xy();
    let mut ball = to_kick_direction * kick.ball_position.inner.coords;
    let angle_weight = 1.0
        - ramp(
            normalize_angle(kick.direction).abs(),
            parameters.shift_direction_degrees[0].to_radians(),
            parameters.shift_direction_degrees[1].to_radians(),
        );
    let foot_weight = ramp(
        (left.y - right.y).abs(),
        parameters.shift_foot_distance[0],
        parameters.shift_foot_distance[1],
    );
    let [minimum, maximum] = parameters.shift_ball_distance;
    let shift = parameters.shift_distance
        * (ramp(ball.y - left.y, minimum, maximum)
            - (1.0 - ramp(ball.y - right.y, -maximum, -minimum)));
    ball.y += angle_weight * foot_weight * shift;
    Point2::wrap(
        (UnitComplex::new(kick.direction) * ball)
            .cap_magnitude(parameters.ball_position_limit)
            .into(),
    )
}
