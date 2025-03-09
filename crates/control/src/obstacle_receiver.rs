use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{MainOutput, PerceptionInput};
use linear_algebra::{Isometry2, Point2};
use spl_network_messages::{GamePhase, HulkMessage, SubState};
use types::{
    filtered_game_controller_state::FilteredGameControllerState, messages::IncomingMessage,
};

#[derive(Deserialize, Serialize)]
pub struct ObstacleReceiver {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    filtered_game_controller_state:
        Input<Option<FilteredGameControllerState>, "filtered_game_controller_state?">,
    ground_to_field: RequiredInput<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    network_message: PerceptionInput<Option<IncomingMessage>, "SplNetwork", "filtered_message?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub network_robot_obstacles: MainOutput<Vec<Point2<Ground>>>,
}

impl ObstacleReceiver {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let messages = context
            .network_message
            .persistent
            .values()
            .flat_map(|messages| messages.iter().filter_map(|message| (*message)))
            .filter_map(|message| match message {
                IncomingMessage::Spl(message) => Some(*message),
                _ => None,
            });
        let mut network_robot_obstacles = Vec::new();
        for message in messages.clone() {
            let pose = match message {
                HulkMessage::Striker(striker_message) => striker_message.pose,
                HulkMessage::Loser(loser_message) => loser_message.pose,
                HulkMessage::VisualReferee(_) => continue,
            };
            let sender_position = context.ground_to_field.inverse() * pose.position();
            network_robot_obstacles.push(sender_position);
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
                    network_robot_obstacles: Vec::new().into(),
                });
            }
        }

        Ok(MainOutputs {
            network_robot_obstacles: network_robot_obstacles.into(),
        })
    }
}
