use std::sync::Arc;

use booster::LowCommand;
use tokio::sync::{mpsc, Mutex};

use super::messages::SimulationData;

pub struct Connection {
    pub(super) low_command_receiver: Arc<Mutex<mpsc::Receiver<LowCommand>>>,
    pub(super) websocket_sender: mpsc::Sender<SimulationData>,
}
