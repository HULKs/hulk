use std::{net::SocketAddr, time::Duration};

use color_eyre::{Result, eyre::Context};
use types::parameters::BehaviorParameters;

pub const DEFAULT_TICK_DURATION: Duration = Duration::from_millis(10);

#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct SimulationConfig {
    pub walk_translation_speed: f32,
    pub walk_rotation_speed: f32,
    pub walk_with_velocity_scale: f32,
    pub kick_ball_speed_rumpelstilzchen: f32,
    pub kick_ball_speed_schlong: f32,
    pub kick_cooldown: Duration,
    pub ball_friction_per_second: f32,
    pub ball_visibility_range: f32,
    /// Total field of view used for filtering ball and robot perception.
    /// `visible = object_angle >= fov / 2`
    pub visibility_field_of_view: f32,
    pub head_yaw_minimum: f32,
    pub head_yaw_maximum: f32,
    pub head_yaw_velocity: f32,
    pub head_scan_period: Duration,
    pub head_glance_angle: f32,
    pub robot_radius: f32,
    pub kick_radius: f32,
    pub remaining_amount_of_messages: Option<u16>,
    pub game_controller_address: Option<SocketAddr>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            walk_translation_speed: 2.0,
            walk_rotation_speed: 3.0,
            walk_with_velocity_scale: 1.0,
            kick_ball_speed_rumpelstilzchen: 2.0,
            kick_ball_speed_schlong: 3.0,
            kick_cooldown: Duration::from_millis(750),
            ball_friction_per_second: 0.6,
            ball_visibility_range: 4.0,
            visibility_field_of_view: std::f32::consts::FRAC_PI_2,
            head_yaw_minimum: -0.785,
            head_yaw_maximum: 0.785,
            head_yaw_velocity: 0.4,
            head_scan_period: Duration::from_secs(4),
            head_glance_angle: 0.25,
            robot_radius: 0.25,
            kick_radius: 0.25,
            remaining_amount_of_messages: Some(u16::MAX),
            game_controller_address: None,
        }
    }
}

pub fn default_behavior_parameters() -> Result<BehaviorParameters> {
    json5::from_str(include_str!(
        "../../../etc/parameters/ros_z/base/behavior_node.json5"
    ))
    .wrap_err("failed to parse behavior parameters")
}
