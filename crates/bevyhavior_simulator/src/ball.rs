use std::time::SystemTime;

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use hsl_network_messages::{GameState, Team};
use linear_algebra::{Isometry2, Point2, Vector2, distance};
use serde::{Deserialize, Serialize};
use types::{
    field_dimensions::{GlobalFieldSide, Side},
    world_state::BallState,
};

use crate::{
    behavior_tree_simulator::{SimulatorClock, SimulatorFieldDimensions},
    config::SimulationConfig,
    coordinates::point_world_to_field,
    game_controller::SimulatorGameState,
    robot::{SimulatorGroundToWorld, SimulatorRobot, SimulatorRobotId},
};

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SimulatorBall {
    pub state: Option<SimulatedBall>,
    pub last_touch_team: Option<Team>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SimulatedBall {
    pub position: Point2<World>,
    pub velocity: Vector2<World>,
    pub field_side: Side,
}

impl SimulatedBall {
    pub fn to_ball_state(
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

pub fn move_ball(
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

pub fn update_ball_last_touch_from_robot_contacts(
    game_state: Res<SimulatorGameState>,
    config: Res<SimulationConfig>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    mut ball: ResMut<SimulatorBall>,
    robots: Query<(&SimulatorRobot, &SimulatorGroundToWorld)>,
) {
    if game_state.game_controller_state.game_state != GameState::Playing {
        return;
    }

    let Some(simulated_ball) = ball.state else {
        return;
    };

    let contact_radius = config.robot_radius + field_dimensions.0.ball_radius;
    if let Some(team) = last_touch_team_from_robot_contact(
        simulated_ball.position,
        contact_radius,
        robots.iter().map(|(robot, ground_to_world)| {
            (robot.id(), ground_to_world.ground_to_world.translation())
        }),
    ) {
        ball.last_touch_team = Some(team);
    }
}

fn last_touch_team_from_robot_contact(
    ball_position: Point2<World>,
    contact_radius: f32,
    robot_positions: impl IntoIterator<Item = (SimulatorRobotId, Point2<World>)>,
) -> Option<Team> {
    robot_positions
        .into_iter()
        .filter_map(|(robot_id, robot_position)| {
            let contact_distance = distance(robot_position, ball_position);
            (contact_distance <= contact_radius).then_some((robot_id, contact_distance))
        })
        .min_by(|(first_id, first_distance), (second_id, second_distance)| {
            first_distance
                .total_cmp(second_distance)
                .then_with(|| first_id.cmp(second_id))
        })
        .map(|(robot_id, _)| robot_id.team)
}
