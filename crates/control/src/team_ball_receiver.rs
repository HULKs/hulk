use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Field;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use linear_algebra::Vector2;
use spl_network_messages::{GamePhase, HulkMessage, SubState};
use types::{
    ball_position::BallPosition, cycle_time::CycleTime,
    filtered_game_controller_state::FilteredGameControllerState, messages::IncomingMessage,
    players::Players,
};

#[derive(Deserialize, Serialize)]
pub struct TeamBallReceiver {
    received_balls: Players<Option<BallPosition<Field>>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,

    maximum_age: Parameter<Duration, "team_ball.maximum_age">,

    team_balls: AdditionalOutput<Players<Option<BallPosition<Field>>>, "team_balls">,
}

#[context]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition<Field>>>,
}

impl TeamBallReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            received_balls: Players::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let messages = get_spl_messages(&context.network_message.persistent);
        for (time, message) in messages {
            self.process_message(time, message);
        }

        // Ignore everything during penalty_*
        if let Some(game_controller_state) = context.filtered_game_controller_state {
            let in_penalty_shootout = matches!(
                game_controller_state.game_phase,
                GamePhase::PenaltyShootout { .. }
            );
            let in_penalty_kick = game_controller_state.sub_state == Some(SubState::PenaltyKick);

            if in_penalty_shootout || in_penalty_kick {
                return Ok(MainOutputs {
                    team_ball: None.into(),
                });
            }
        }

        let team_ball =
            self.get_best_received_ball(context.cycle_time.start_time, *context.maximum_age);

        context.team_balls.fill_if_subscribed(|| {
            self.received_balls.map(|ball| {
                ball.filter(|ball| {
                    context
                        .cycle_time
                        .start_time
                        .duration_since(ball.last_seen)
                        .expect("time ran backwards")
                        < *context.maximum_age
                })
            })
        });

        Ok(MainOutputs {
            team_ball: team_ball.into(),
        })
    }

    fn process_message(&mut self, time: SystemTime, message: HulkMessage) {
        let (player, ball) = match message {
            HulkMessage::Striker(striker_message) => (
                striker_message.player_number,
                Some(BallPosition {
                    position: striker_message.ball_position.position,
                    velocity: Vector2::zeros(),
                    last_seen: time - striker_message.ball_position.age,
                }),
            ),
            HulkMessage::Loser(loser_message) => (loser_message.player_number, None),
            HulkMessage::VisualReferee(_) => return,
        };
        self.received_balls[player] = ball;
    }

    fn get_best_received_ball(
        &self,
        now: SystemTime,
        trust_duration: Duration,
    ) -> Option<BallPosition<Field>> {
        self.received_balls
            .iter()
            .filter_map(|(_player_number, ball)| *ball)
            .max_by_key(|ball| ball.last_seen)
            .filter(|ball| {
                now.duration_since(ball.last_seen)
                    .expect("time ran backwards")
                    < trust_duration
            })
    }
}

fn get_spl_messages<'a>(
    persistent_messages: &'a BTreeMap<SystemTime, Vec<Option<&'_ IncomingMessage>>>,
) -> impl Iterator<Item = (SystemTime, HulkMessage)> + 'a {
    persistent_messages
        .iter()
        .flat_map(|(time, messages)| {
            messages
                .iter()
                .filter_map(|message| Some((*time, (*message)?)))
        })
        .filter_map(|(time, message)| match message {
            IncomingMessage::Spl(message) => Some((time, *message)),
            _ => None,
        })
}
