use std::sync::Arc;

use spl_network::{GameControllerReturnMessage, SplMessage};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};

#[derive(Clone, Debug)]
pub struct MessageReceivers {
    pub game_controller_return_message_receiver:
        Arc<Mutex<UnboundedReceiver<GameControllerReturnMessage>>>,
    pub spl_message_receiver: Arc<Mutex<UnboundedReceiver<SplMessage>>>,
}

impl Default for MessageReceivers {
    fn default() -> Self {
        // This can only happen if someone deserializes a value into Option<MessageReceivers>
        // but MessageReceivers have #[dont_serialize] in the database.
        // So let's tell the compiler everything will be fine:
        panic!("MessageReceivers cannot be Default constructed");
    }
}
