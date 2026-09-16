use std::{collections::VecDeque, f32::consts::TAU};

use coordinate_systems::{Ground, Robot};
use kinematics::{forward, joints::Joints};
use linear_algebra::{Point2, Point3, Vector2, point};
use types::{joint_limits::JointLimits, robot_command::MotorCommand};

use crate::{
    config::{HISTORY_FRAME_SIZE, HISTORY_LENGTH, LEGS, Parameters, Policy},
    inference::position_targets,
    observation::SensorFrame,
};

pub mod kick;
pub mod walk;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, ros_z::Message)]
pub struct KickRequest {
    pub ball_position: Point2<Ground>,
    pub ball_velocity: Vector2<Ground>,
    pub direction: f32,
    pub target_speed: f32,
    pub strong: bool,
    pub quick: bool,
}

impl KickRequest {
    pub fn is_finite(self) -> bool {
        self.ball_position
            .inner
            .iter()
            .chain(self.ball_velocity.inner.iter())
            .copied()
            .chain([self.direction, self.target_speed])
            .all(f32::is_finite)
            && self.target_speed >= 0.0
    }
}

pub struct Locomotion {
    pub(crate) parameters: std::sync::Arc<Parameters>,
    history: VecDeque<[f32; HISTORY_FRAME_SIZE]>,
    previous_target: Joints<f32>,
    pub phase: f32,
    pub frequency_offset: f32,
    previous_ball: Option<Point2<Ground>>,
}

impl Locomotion {
    pub fn new(
        sensor: &SensorFrame,
        parameters: std::sync::Arc<Parameters>,
        joints: &JointLimits,
    ) -> Self {
        Self {
            history: VecDeque::from(vec![
                walk::history_frame(
                    sensor,
                    &sensor.last_commanded_position,
                    true,
                    &parameters,
                    joints
                );
                HISTORY_LENGTH
            ]),
            previous_target: sensor.last_commanded_position,
            phase: 0.0,
            frequency_offset: parameters.locomotion.initial_frequency_offset,
            parameters,
            previous_ball: None,
        }
    }

    pub fn advance(&mut self, seconds: f32, standing: bool) {
        if standing {
            self.phase = 0.0;
        } else {
            let limit = self.parameters.locomotion.frequency_offset_limit;
            self.phase = (self.phase
                + seconds
                    * (self.parameters.locomotion.base_frequency
                        + self.frequency_offset.clamp(-limit, limit)))
            .rem_euclid(1.0);
        }
    }

    pub fn record_walk_sample(&mut self, sensor: &SensorFrame, joints: &JointLimits) {
        self.record_history(sensor, joints);
        self.previous_ball = None;
    }

    pub fn record_kick_sample(
        &mut self,
        sensor: &SensorFrame,
        request: KickRequest,
        joints: &JointLimits,
    ) -> (Point2<Ground>, Point2<Ground>) {
        self.record_history(sensor, joints);
        let ball = kick::shifted_ball(sensor, request, &self.parameters.kick);
        let previous = self
            .previous_ball
            .filter(|previous| (ball - *previous).norm() <= self.parameters.kick.ball_jump_distance)
            .unwrap_or(ball);
        self.previous_ball = Some(ball);
        (ball, previous)
    }

    fn record_history(&mut self, sensor: &SensorFrame, joints: &JointLimits) {
        self.history.pop_front();
        self.history.push_back(walk::history_frame(
            sensor,
            &self.previous_target,
            false,
            &self.parameters,
            joints,
        ));
    }

    fn phase_encoding(&self, standing: bool) -> [f32; 2] {
        if standing {
            [0.0, 0.0]
        } else {
            [(TAU * self.phase).cos(), (TAU * self.phase).sin()]
        }
    }

    pub fn decode(
        &mut self,
        policy: Policy,
        actions: &[f32],
        sensor: &SensorFrame,
    ) -> Joints<MotorCommand> {
        let offset = policy.offset(&self.parameters);
        let (kp, kd) = policy.gains(&self.parameters);
        let action_limit = policy.action_limit(&self.parameters);
        let mut position = sensor.last_commanded_position;
        for (index, joint) in LEGS.into_iter().enumerate() {
            position[joint] = actions[index].clamp(-action_limit, action_limit) + offset[joint];
            // RLWalkPhase::calcJoints retains these targets before downstream composition/clipping.
            self.previous_target[joint] = position[joint];
        }
        let limit = self.parameters.locomotion.frequency_offset_limit;
        self.frequency_offset = actions[LEGS.len()].clamp(-limit, limit);
        position_targets(position, kp, kd)
    }
}

pub fn leg(position: &Joints<f32>, left: bool) -> (Point3<Robot>, Point3<Robot>) {
    if left {
        let angles = &position.left_leg;
        let tibia_to_robot = forward::left_pelvis_to_robot(angles)
            * forward::left_hip_to_left_pelvis(angles)
            * forward::left_thigh_to_left_hip(angles)
            * forward::left_tibia_to_left_thigh(angles);
        let foot_to_robot = tibia_to_robot
            * forward::left_ankle_to_left_tibia(angles)
            * forward::left_foot_to_left_ankle(angles);
        (
            foot_to_robot * point![0.026, 0.0, -0.038],
            tibia_to_robot.translation(),
        )
    } else {
        let angles = &position.right_leg;
        let tibia_to_robot = forward::right_pelvis_to_robot(angles)
            * forward::right_hip_to_right_pelvis(angles)
            * forward::right_thigh_to_right_hip(angles)
            * forward::right_tibia_to_right_thigh(angles);
        let foot_to_robot = tibia_to_robot
            * forward::right_ankle_to_right_tibia(angles)
            * forward::right_foot_to_right_ankle(angles);
        (
            foot_to_robot * point![0.026, 0.0, -0.038],
            tibia_to_robot.translation(),
        )
    }
}
