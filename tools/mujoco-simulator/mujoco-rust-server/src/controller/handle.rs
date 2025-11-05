use booster::LowCommand;
use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use simulation_message::ConnectionInfo;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::controller::messages::{ControlCommand, SimulationData};

#[derive(Clone)]
pub struct ControllerHandle {
    pub(super) sender: mpsc::Sender<ControlCommand>,
}

impl ControllerHandle {
    pub async fn connect(&self, connection_info: ConnectionInfo) -> Result<ConnectionHandle> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(ControlCommand::Connect {
                sender: tx,
                connection_info,
            })
            .await
            .wrap_err("failed to send Connect to controller")?;
        rx.await.wrap_err("channel closed")
    }
}

pub struct ConnectionHandle {
    pub(super) id: Uuid,
    pub(super) control_sender: mpsc::Sender<ControlCommand>,
    pub(super) low_command_sender: mpsc::Sender<LowCommand>,
    pub(super) websocket_receiver: mpsc::Receiver<SimulationData>,
}

impl ConnectionHandle {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn send_low_command(&self, low_command: LowCommand) -> Result<()> {
        self.low_command_sender
            .send(low_command)
            .await
            .wrap_err("failed to send LowCommand")
    }

    pub async fn disconnect(self) {
        let (tx, rx) = oneshot::channel();
        if let Err(error) = self
            .control_sender
            .send(ControlCommand::Disconnect {
                id: self.id,
                sender: tx,
            })
            .await
        {
            log::error!("failed to send Disconnect command: {error}")
        }

        if let Err(error) = rx.await {
            log::error!("channel closed: {error}")
        }
    }

    pub async fn receive_data(&mut self) -> Result<SimulationData> {
        self.websocket_receiver
            .recv()
            .await
            .wrap_err("channel closed")
    }
}
