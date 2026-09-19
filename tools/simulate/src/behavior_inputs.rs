//! Ground-truth substitutes for perception/localization, with real downstream composers.
use std::f32::consts::PI;

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Point2, point, vector};
use ros_z::{cache::Cache, prelude::*, qos::QosDurability, time::Time};
use types::{
    ball_position::BallPosition, field_dimensions::GlobalFieldSide,
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState, obstacles::Obstacle, primary_state::PrimaryState,
};

pub struct BehaviorInputs {
    pose: Publisher<Isometry2<Ground, Field>>,
    ball: Publisher<Option<BallPosition<Ground>>>,
    visual_ball: Publisher<Option<BallPosition<Ground>>>,
    obstacles: Publisher<Vec<Obstacle>>,
    interest: Publisher<Point2<Ground>>,
    primary: Publisher<PrimaryState>,
    player: Cache<PlayerNumber>,
}

impl BehaviorInputs {
    pub async fn new(node: &Node) -> Result<Self> {
        let retained = QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        };
        Ok(Self {
            pose: node.publisher("ground_to_field").build().await?,
            ball: node.publisher("ball_filter/ball_position").build().await?,
            visual_ball: node.publisher("visual_kick/ball_position").build().await?,
            obstacles: node.publisher("obstacles").build().await?,
            interest: node.publisher("position_of_interest").build().await?,
            primary: node
                .publisher("primary_state")
                .qos(retained)
                .build()
                .await?,
            player: node
                .subscriber("player_number")
                .qos(retained)
                .cache(1)
                .build()
                .await?,
        })
    }

    pub async fn publish_game(&self, game: &FilteredGameControllerState) -> Result<()> {
        // Wait for the configured player number rather than applying another player's penalty.
        if let Some(player) = self.player.get_latest() {
            self.primary.publish(&primary_state(game, *player)).await?;
        }
        Ok(())
    }

    pub async fn publish(
        &self,
        ground_to_world: nalgebra::Isometry3<f32>,
        ball: Option<([f64; 3], [f64; 3])>,
        obstacle_positions: Vec<[f64; 3]>,
        side: GlobalFieldSide,
        time: Time,
    ) -> Result<()> {
        let pose = ground_to_field(ground_to_world, side);
        let ball = ball
            .map(|(position, velocity)| ball_in_ground(ground_to_world, position, velocity, time));
        let inverse = ground_to_world.inverse();
        let obstacles = obstacle_positions
            .into_iter()
            .map(|position| {
                let p = inverse * nalgebra::Point3::from(position.map(|v| v as f32));
                // Conservative K1 collision footprint; physical contacts still use the full MJCF.
                Obstacle::robot(point![p.x, p.y], 0.25, 0.3)
            })
            .collect();
        self.pose.publish_with_source_time(&pose, time).await?;
        self.ball.publish_with_source_time(&ball, time).await?;
        self.visual_ball
            .publish_with_source_time(&ball, time)
            .await?;
        self.obstacles
            .publish_with_source_time(&obstacles, time)
            .await?;
        self.interest
            .publish_with_source_time(&ball.map_or(point![1.0, 0.0], |b| b.position), time)
            .await?;
        Ok(())
    }
}

fn primary_state(game: &FilteredGameControllerState, player: PlayerNumber) -> PrimaryState {
    if game.game_state == FilteredGameState::Finished {
        return PrimaryState::Finished;
    }
    if game.penalties[player].is_some() {
        return PrimaryState::Penalized;
    }
    match game.game_state {
        FilteredGameState::Initial => PrimaryState::Initial,
        FilteredGameState::Ready => PrimaryState::Ready,
        FilteredGameState::Set => PrimaryState::Set,
        FilteredGameState::Playing { .. } => PrimaryState::Playing,
        FilteredGameState::Stop => PrimaryState::Stop,
        FilteredGameState::Finished => PrimaryState::Finished,
    }
}

fn ground_to_field(
    pose: nalgebra::Isometry3<f32>,
    side: GlobalFieldSide,
) -> Isometry2<Ground, Field> {
    let yaw = pose.rotation.euler_angles().2;
    let ground = nalgebra::Isometry2::new(
        nalgebra::vector![pose.translation.x, pose.translation.y],
        yaw,
    );
    let rotation = if side == GlobalFieldSide::Home {
        0.0
    } else {
        PI
    };
    Isometry2::wrap(nalgebra::Isometry2::new(nalgebra::Vector2::zeros(), rotation) * ground)
}

fn ball_in_ground(
    pose: nalgebra::Isometry3<f32>,
    position: [f64; 3],
    velocity: [f64; 3],
    time: Time,
) -> BallPosition<Ground> {
    let inverse = pose.inverse();
    let position = inverse * nalgebra::Point3::from(position.map(|v| v as f32));
    let velocity = inverse.rotation * nalgebra::Vector3::from(velocity.map(|v| v as f32));
    BallPosition {
        position: point![position.x, position.y],
        velocity: vector![velocity.x, velocity.y],
        last_seen: time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_truth_rotates_positions_velocities_and_field_side() {
        let pose = nalgebra::Isometry3::new(
            nalgebra::vector![2.0, 3.0, 0.0],
            nalgebra::vector![0.0, 0.0, PI / 2.0],
        );
        let time = Time::from_nanos(42);
        let ball = ball_in_ground(pose, [2.0, 4.0, 0.1], [0.0, 2.0, 0.0], time);
        assert!((ball.position - point![1.0, 0.0]).norm() < 1e-5);
        assert!((ball.velocity - vector![2.0, 0.0]).norm() < 1e-5);
        assert_eq!(ball.last_seen, time);
        for (side, sign) in [(GlobalFieldSide::Home, 1.0), (GlobalFieldSide::Away, -1.0)] {
            let field = ground_to_field(pose, side) * ball.position;
            assert!((field - point![2.0 * sign, 4.0 * sign]).norm() < 1e-5);
        }
    }

    #[test]
    fn game_state_uses_configured_player_penalty() {
        let mut game = FilteredGameControllerState {
            game_state: FilteredGameState::Ready,
            ..Default::default()
        };
        assert_eq!(
            primary_state(&game, PlayerNumber::Three),
            PrimaryState::Ready
        );
        game.penalties[PlayerNumber::Three] = Some(hsl_network_messages::Penalty::PickUp {
            remaining: std::time::Duration::from_secs(30),
        });
        assert_eq!(
            primary_state(&game, PlayerNumber::Three),
            PrimaryState::Penalized
        );
        assert_eq!(primary_state(&game, PlayerNumber::Two), PrimaryState::Ready);
        game.game_state = FilteredGameState::Finished;
        assert_eq!(
            primary_state(&game, PlayerNumber::Three),
            PrimaryState::Finished
        );
    }
}
