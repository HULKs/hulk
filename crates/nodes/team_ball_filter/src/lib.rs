use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use hsl_network_messages::{GamePhase, SubState};
use serde::{Deserialize, Serialize};

use coordinate_systems::Field;
use ros_z::prelude::*;
use types::{
    ball_position::BallPosition, filtered_game_controller_state::FilteredGameControllerState,
    players::Players, time_wrapper::TimeWrapper, world_state::PlayerState,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub maximum_age: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("team_ball_filter").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("team_ball")?;
    let filtered_game_controller_state_sub = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .build()
        .await?;
    let player_states_sub = node
        .subscriber::<Players<Option<TimeWrapper<PlayerState>>>>("player_states")
        .build()
        .await?;
    let team_ball_pub = node
        .publisher::<Option<BallPosition<Field>>>("team_ball")
        .build()
        .await?;

    let mut team_ball_filter = TeamBallFilter::default();
    let mut filtered_game_controller_state = None;

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        tokio::select! {
            received_player_states = player_states_sub.recv() => {
                team_ball_filter.update_received_balls(received_player_states?);
            }
            received_filtered_game_controller_state = filtered_game_controller_state_sub.recv() => {
                filtered_game_controller_state = Some(received_filtered_game_controller_state?);
            }
        }

        let now = node.clock().now();
        if filtered_game_controller_state
            .as_ref()
            .is_some_and(is_penalty_phase_or_sub_state)
        {
            team_ball_pub.publish(&None).await?;
            continue;
        }

        let team_ball = team_ball_filter.get_best_received_ball(now, parameters.maximum_age);
        team_ball_pub.publish(&team_ball).await?;
    }
}

#[derive(Default)]
struct TeamBallFilter {
    received_balls: Players<Option<BallPosition<Field>>>,
}

impl TeamBallFilter {
    fn update_received_balls(&mut self, player_states: Players<Option<TimeWrapper<PlayerState>>>) {
        self.received_balls = player_states
            .map(|player_state| player_state.and_then(|state| state.inner.ball_position));
    }

    fn get_best_received_ball(
        &self,
        now: ros_z::time::Time,
        maximum_age: Duration,
    ) -> Option<BallPosition<Field>> {
        self.received_balls
            .iter()
            .filter_map(|(_player_number, ball)| *ball)
            .max_by_key(|ball| ball.last_seen)
            .filter(|ball| ball.age_at(now).is_some_and(|age| age < maximum_age))
    }
}

fn is_penalty_phase_or_sub_state(game_controller_state: &FilteredGameControllerState) -> bool {
    matches!(
        game_controller_state.game_phase,
        GamePhase::PenaltyShootout { .. }
    ) || game_controller_state.sub_state == Some(SubState::PenaltyKick)
}
