use std::collections::VecDeque;

use ::kinematics::joints::Joints;
use coordinate_systems::Ground;
use linear_algebra::Vector2;

use super::Locomotion;
use crate::{
    config::{HISTORY_FRAME_SIZE, HISTORY_LENGTH, LEGS, Parameters, Policy, clip_measurement},
    observation::SensorFrame,
};

pub struct Observation<'a> {
    history: &'a VecDeque<[f32; HISTORY_FRAME_SIZE]>,
    pub linear_velocity: Vector2<Ground>,
    pub angular_velocity: f32,
    pub phase: [f32; 2],
    pub joint_velocity: Joints<f32>,
    pub frequency_offset: f32,
    joint_velocity_scale: f32,
}

impl<'a> Observation<'a> {
    pub fn new(
        state: &'a Locomotion,
        joint_velocity: &Joints<f32>,
        standing: bool,
        linear_velocity: Vector2<Ground>,
        angular_velocity: f32,
    ) -> Self {
        Self {
            history: &state.history,
            joint_velocity_scale: state.parameters.observation.joint_velocity_scale,
            linear_velocity: if standing {
                Vector2::zeros()
            } else {
                linear_velocity
            },
            angular_velocity: if standing { 0.0 } else { angular_velocity },
            phase: state.phase_encoding(standing),
            joint_velocity: *joint_velocity,
            frequency_offset: state.frequency_offset,
        }
    }

    pub fn to_tensor(&self) -> [f32; 344] {
        let mut input = [0.0; 344];
        let (history, current) = input.split_at_mut(HISTORY_LENGTH * HISTORY_FRAME_SIZE);
        for (slot, frame) in history
            .as_chunks_mut::<HISTORY_FRAME_SIZE>()
            .0
            .iter_mut()
            .zip(self.history)
        {
            *slot = *frame;
        }
        current[..3].copy_from_slice(&[
            self.linear_velocity.x(),
            self.linear_velocity.y(),
            self.angular_velocity,
        ]);
        current[3..5].copy_from_slice(&self.phase);
        current[5..17].copy_from_slice(
            &LEGS.map(|joint| self.joint_velocity[joint] * self.joint_velocity_scale),
        );
        current[17] = self.frequency_offset;
        // The remaining six slots are trained padding, not additional features.
        input
    }
}

/// Initial history omits gyro. Both trailing zero slots are part of the trained frame.
pub(super) fn history_frame(
    sensor: &SensorFrame,
    previous_target: &Joints<f32>,
    initializing: bool,
    parameters: &Parameters,
) -> [f32; HISTORY_FRAME_SIZE] {
    let mut frame = [0.0; HISTORY_FRAME_SIZE];
    frame[..3].copy_from_slice(&sensor.gravity());
    if !initializing {
        frame[3..6].copy_from_slice(sensor.gyro.inner.as_slice());
    }
    let measured_position = clip_measurement(sensor.position, parameters.joint_limits);
    let offset = Policy::Walk.offset(parameters);
    frame[6..18].copy_from_slice(&LEGS.map(|joint| measured_position[joint] - offset[joint]));
    frame[18..30].copy_from_slice(&LEGS.map(|joint| previous_target[joint] - offset[joint]));
    frame
}
