use std::time::SystemTime;

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use linear_algebra::{Isometry2, Point2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::{GlobalFieldSide, Side},
    world_state::BallState,
};

use crate::{
    behavior_tree_simulator::SimulatorClock, config::SimulationConfig,
    coordinates::point_world_to_field,
};

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SimulatorBall {
    pub state: Option<SimulatedBall>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SimulatedBall {
    pub position: Point2<World>,
    pub velocity: Vector2<World>,
    pub field_side: Side,
}

impl SimulatedBall {
    pub(crate) fn to_ball_state(
        self,
        ground_to_world: Isometry2<Ground, World>,
        global_field_side: GlobalFieldSide,
        now: SystemTime,
    ) -> BallState {
        let ball_in_field = point_world_to_field(self.position, global_field_side);
        BallState {
            ball_in_ground: ground_to_world.inverse() * self.position,
            ball_in_field,
            ball_in_ground_velocity: ground_to_world.inverse() * self.velocity,
            last_seen_ball: now,
            field_side: self.field_side,
        }
    }
}

pub(crate) fn update_ball_kinematics(
    clock: Res<SimulatorClock>,
    config: Res<SimulationConfig>,
    mut ball: ResMut<SimulatorBall>,
) {
    let Some(ball) = &mut ball.state else {
        return;
    };
    let dt = clock.tick_duration.as_secs_f32();
    ball.position += ball.velocity * dt;
    ball.velocity *= (1.0 - config.ball_friction_per_second * dt).clamp(0.0, 1.0);
}

pub(crate) fn perceived_ball_from_pose(
    ball: Option<SimulatedBall>,
    ground_to_world: Isometry2<Ground, World>,
    global_field_side: GlobalFieldSide,
    now: SystemTime,
    config: &SimulationConfig,
) -> Option<BallState> {
    let ball = ball?;
    let ball_in_ground = ground_to_world.inverse() * ball.position;
    let distance = ball_in_ground.coords().norm();
    if distance > config.ball_visibility_range {
        return None;
    }

    let angle = ball_in_ground.coords().angle(&Vector2::x_axis());
    if angle.abs() > config.ball_visibility_angle / 2.0 {
        return None;
    }

    Some(ball.to_ball_state(ground_to_world, global_field_side, now))
}
