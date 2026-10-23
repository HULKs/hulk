use std::{num::NonZeroUsize, time::Duration};

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use hsl_network_messages::PlayerNumber;
use kinematics::joints::head::HeadJoints;
use linear_algebra::{Isometry2, Point2, Vector2};
use ros_z::{
    cache::Cache,
    prelude::*,
    qos::{QosDurability, QosHistory},
};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    heatmap::SEARCH_CELL_SIZE_TOPIC,
    obstacles::Obstacle,
    players::Players,
    primary_state::PrimaryState,
    time_wrapper::TimeWrapper,
    world_state::PlayerState,
};

pub(super) struct Topics {
    pub field_dimensions_sub: Subscriber<FieldDimensions>,
    pub ball_position_sub: Subscriber<Option<BallPosition<Ground>>>,
    pub player_number_cache: Cache<PlayerNumber>,
    pub goalkeeper_player_number_cache: Cache<PlayerNumber>,
    pub hsl_state_message_send_interval_cache: Cache<Option<Duration>>,
    pub hypothetical_ball_positions_sub: Subscriber<Vec<HypotheticalBallPosition<Ground>>>,
    pub ground_to_field_cache: Cache<Isometry2<Ground, Field>>,
    pub primary_state_cache: Cache<PrimaryState>,
    pub filtered_game_controller_state_sub: Subscriber<FilteredGameControllerState>,
    pub obstacles_cache: Cache<Vec<Obstacle>>,
    pub head_joints_command_cache: Cache<HeadJoints<f32>>,
    pub last_received_player_states_sub: Subscriber<Players<Option<TimeWrapper<PlayerState>>>>,
    pub additional_heatmap_pub: Publisher<types::heatmap::Heatmap>,
    pub suggested_search_position_pub: Publisher<Option<Point2<Field>>>,
    pub cell_size_pub: Publisher<Vector2<Field>>,
}

impl Topics {
    pub(super) async fn build(node: &Node) -> Result<Self> {
        let field_dimensions_sub = node
            .subscriber::<FieldDimensions>("field_dimensions")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .build()
            .await?;
        let ball_position_sub = node
            .subscriber::<Option<BallPosition<Ground>>>("ball_filter/ball_position")
            .build()
            .await?;
        let player_number_cache = node
            .subscriber::<PlayerNumber>("player_number")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .cache(1)
            .build()
            .await?;
        let goalkeeper_player_number_cache = node
            .subscriber::<PlayerNumber>("behavior/goalkeeper_player_number")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(NonZeroUsize::MIN),
                ..Default::default()
            })
            .cache(1)
            .build()
            .await?;
        let hsl_state_message_send_interval_cache = node
            .subscriber::<Option<Duration>>("hsl_state_message_send_interval")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(NonZeroUsize::MIN),
                ..Default::default()
            })
            .cache(1)
            .build()
            .await?;
        let hypothetical_ball_positions_sub = node
            .subscriber::<Vec<HypotheticalBallPosition<Ground>>>(
                "ball_filter/hypothetical_ball_positions",
            )
            .build()
            .await?;
        let ground_to_field_cache = node
            .subscriber::<Isometry2<Ground, Field>>("ground_to_field")
            .cache(10)
            .build()
            .await?;
        let primary_state_cache = node
            .subscriber::<PrimaryState>("primary_state")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .cache(1)
            .build()
            .await?;
        let filtered_game_controller_state_sub = node
            .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
            .build()
            .await?;
        let obstacles_cache = node
            .subscriber::<Vec<Obstacle>>("obstacles")
            .cache(1)
            .build()
            .await?;
        let head_joints_command_cache = node
            .subscriber::<HeadJoints<f32>>("head_joints_command")
            .cache(1)
            .build()
            .await?;
        let last_received_player_states_sub = node
            .subscriber::<Players<Option<TimeWrapper<PlayerState>>>>("last_received_player_states")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(NonZeroUsize::MIN),
                ..Default::default()
            })
            .build()
            .await?;
        let additional_heatmap_pub = node
            .publisher::<types::heatmap::Heatmap>("ball_search_heatmap")
            .build()
            .await?;
        let suggested_search_position_pub = node
            .publisher::<Option<Point2<Field>>>("suggested_search_position")
            .build()
            .await?;

        let cell_size_pub = node
            .publisher::<Vector2<Field>>(SEARCH_CELL_SIZE_TOPIC)
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                history: QosHistory::KeepLast(NonZeroUsize::MIN),
                ..Default::default()
            })
            .build()
            .await?;
        Ok(Self {
            field_dimensions_sub,
            ball_position_sub,
            player_number_cache,
            goalkeeper_player_number_cache,
            hsl_state_message_send_interval_cache,
            hypothetical_ball_positions_sub,
            ground_to_field_cache,
            primary_state_cache,
            filtered_game_controller_state_sub,
            obstacles_cache,
            head_joints_command_cache,
            last_received_player_states_sub,
            additional_heatmap_pub,
            suggested_search_position_pub,
            cell_size_pub,
        })
    }

    pub(super) async fn drain_disabled_inputs(&self) -> Result<()> {
        while self.ball_position_sub.is_ready() {
            self.ball_position_sub.recv().await?;
        }
        while self.hypothetical_ball_positions_sub.is_ready() {
            self.hypothetical_ball_positions_sub.recv().await?;
        }
        while self.last_received_player_states_sub.is_ready() {
            self.last_received_player_states_sub.recv().await?;
        }
        while self.filtered_game_controller_state_sub.is_ready() {
            self.filtered_game_controller_state_sub.recv().await?;
        }
        Ok(())
    }
}
