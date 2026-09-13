use std::time::Duration;

use kinematics::joints::head::HeadJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::parameters::ImageRegionParameters;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub kp: HeadJoints<f32>,
    pub kd: HeadJoints<f32>,

    pub maximum_velocity: HeadJoints<f32>,
    pub maximum_defender_velocity: HeadJoints<f32>,
    pub maximum_pitch: f32,
    pub minimum_pitch: f32,
    pub maximum_yaw: f32,
    pub minimum_yaw: f32,
    pub injected_head_joints: Option<HeadJoints<f32>>,

    pub glance_angle: f32,
    pub image_region_parameters: ImageRegionParameters,
    pub glance_direction_toggle_interval: Duration,

    pub look_around_timeout: Duration,
    pub quick_search_timeout: Duration,

    pub middle_positions: HeadJoints<f32>,
    pub left_positions: HeadJoints<f32>,
    pub right_positions: HeadJoints<f32>,
    pub halfway_left_positions: HeadJoints<f32>,
    pub halfway_right_positions: HeadJoints<f32>,
    pub initial_left_positions: HeadJoints<f32>,
    pub initial_right_positions: HeadJoints<f32>,
}
