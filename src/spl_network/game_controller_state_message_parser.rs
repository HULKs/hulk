use std::convert::TryInto;

use log::warn;
use spl_network_messages::GameControllerStateMessage;

pub fn parse_game_controller_state_message(message: &[u8]) -> Option<GameControllerStateMessage> {
    match message.try_into() {
        Ok(message) => Some(message),
        Err(error) => {
            warn!(
                "Failed to parse GameController state message (will be discarded): {:?}",
                error
            );
            None
        }
    }
}
