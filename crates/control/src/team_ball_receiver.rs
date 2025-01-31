use color_eyre::eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Point2, Vector2};
use serde::{Deserialize, Serialize};

use framework::{MainOutput, PerceptionInput};
use spl_network_messages::HulkMessage;
use types::{ball_position::BallPosition, messages::IncomingMessage, players::Players};

#[derive(Deserialize, Serialize)]
pub struct TeamBallReceiver {
    received_balls: Players<Option<BallPosition<Field>>>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
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

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
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
                IncomingMessage::Spl(HulkMessage::Striker(message)) => Some((time, message)),
                _ => None,
            });
        for (time, message) in striker_messages {
            self.received_balls[message.player_number] = Some(BallPosition {
                position: message.ball_position.position,
                velocity: Vector2::zeros(),
                last_seen: time - message.ball_position.age,
            });
        }
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
        let team_ball = self
            .received_balls
            .iter()
            .filter_map(|(_player_number, ball)| *ball)
            .max_by_key(|ball| ball.last_seen);
        Ok(MainOutputs {
            team_ball: team_ball.into(),
            network_robot_obstacles: Vec::new().into(),
        })
    }
}
