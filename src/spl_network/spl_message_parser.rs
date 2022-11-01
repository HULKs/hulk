use std::convert::TryInto;

use log::warn;
use spl_network_messages::SplMessage;

pub fn parse_spl_message(message: &[u8]) -> Option<SplMessage> {
    match message.try_into() {
        Ok(message) => Some(message),
        Err(error) => {
            warn!(
                "Failed to parse SPL message (will be discarded): {:?}",
                error
            );
            None
        }
    }
}
