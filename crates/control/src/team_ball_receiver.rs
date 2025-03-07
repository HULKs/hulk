use std::time::{Duration, SystemTime};

use color_eyre::eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};

use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use spl_network_messages::{GamePhase, HulkMessage, SubState};
use types::{
    ball_position::BallPosition, cycle_time::CycleTime, messages::IncomingMessage, players::Players,
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
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
    network_message_debug:
        AdditionalOutput<Vec<(SystemTime, StrikerMessage)>, "network_message_debug">,

    striker_trusts_team_ball: Parameter<Duration, "spl_network.striker_trusts_team_ball">,

    team_balls: AdditionalOutput<Players<Option<BallPosition<Field>>>, "team_balls">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition<Field>>>,
    pub network_robot_obstacles: MainOutput<Vec<Point2<Ground>>>,
}

impl TeamBallReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            received_balls: Players::default(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let striker_messages = context
            .network_message
            .persistent
            .iter()
            .flat_map(|(time, messages)| {
                messages
                    .iter()
                    .filter_map(|message| Some((*time, (*message)?)))
            })
            .filter_map(|(time, message)| match message {
                IncomingMessage::Spl(message) => Some((time, *message)),
                _ => None,
            });
        for (time, message) in striker_messages.clone() {
            self.process_message(time, message);
        }
        context
            .network_message_debug
            .fill_if_subscribed(|| striker_messages.collect());

        // === Team ball ===
        // if let Some(game_controller_state) = filtered_game_controller_state {
        //     match game_controller_state.game_phase {
        //         GamePhase::PenaltyShootout {
        //             kicking_team: Team::Hulks,
        //         } => return (Role::Striker, false, None),
        //         GamePhase::PenaltyShootout {
        //             kicking_team: Team::Opponent,
        //         } => return (Role::Keeper, false, None),
        //         _ => {}
        //     };
        //     if let Some(SubState::PenaltyKick) = game_controller_state.sub_state {
        //         return (current_role, false, None);
        //     }
        // }

        // if primary_state != PrimaryState::Playing {
        //     match detected_own_team_ball {
        //         None => return (current_role, false, team_ball), Some(own_team_ball) => return (current_role, false, own_team_ball),
        //     }

        // let team_ball_from_spl_message = Some(BallPosition {
        //     position: striker_event.ball_position.position,
        //     velocity: Vector::zeros(),
        //     last_seen: cycle_start_time - striker_event.ball_position.age,
        // });

        // === Obstacles ===
        // let sender_position = ground_to_field.inverse() * spl_message.pose.position();
        // if spl_message.player_number != *context.player_number {
        //     network_robot_obstacles.push(sender_position);
        // }

        let team_ball = self.get_best_received_ball(
            context.cycle_time.start_time,
            context.striker_trusts_team_ball.mul_f32(4.5),
        );

        context.team_balls.fill_if_subscribed(|| {
            self.received_balls.map(|ball| {
                ball.filter(|ball| {
                    context
                        .cycle_time
                        .start_time
                        .duration_since(ball.last_seen)
                        .expect("time ran backwards")
                        < context.striker_trusts_team_ball.mul_f32(4.5)
                })
            })
        });

        Ok(MainOutputs {
            team_ball: team_ball.into(),
            network_robot_obstacles: Vec::new().into(),
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
