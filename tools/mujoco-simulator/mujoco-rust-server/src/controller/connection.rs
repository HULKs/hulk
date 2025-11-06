use std::ops::Range;
use std::sync::Arc;
use std::time::SystemTime;

use color_eyre::eyre::{eyre, Context, ContextCompat};
use color_eyre::Result;
use tokio::sync::oneshot;
use tokio::sync::{mpsc, Mutex};

use booster::LowCommand;
use simulation_message::{ConnectionInfo, OnceTask, PeriodicalTask};

use super::{messages::SimulationData, SimulationTask};

#[derive(Clone, Debug)]
pub struct Connection {
    pub(super) low_command_receiver: Arc<Mutex<mpsc::Receiver<LowCommand>>>,
    pub(super) websocket_sender: mpsc::Sender<SimulationData>,
    pub(super) connection_info: Arc<ConnectionInfo>,
}

impl Connection {
    pub(super) fn initial_tasks(&self) -> Vec<OnceTask> {
        self.connection_info.initial_tasks()
    }

    pub(super) fn due_tasks(&self, range: Range<SystemTime>) -> Vec<PeriodicalTask> {
        self.connection_info.due_tasks(range)
    }

    pub(super) async fn request_low_state(
        &self,
        simulation_sender: &mpsc::Sender<SimulationTask>,
    ) -> Result<()> {
        simulation_sender
            .send(SimulationTask::RequestLowState {
                sender: self.websocket_sender.clone(),
            })
            .await
            .wrap_err("channel closed")
    }

    pub(super) async fn request_rgbd_sensors(
        &self,
        simulation_sender: &mpsc::Sender<SimulationTask>,
    ) -> Result<()> {
        simulation_sender
            .send(SimulationTask::RequestRGBDSensors {
                sender: self.websocket_sender.clone(),
            })
            .await
            .wrap_err("channel closed")
    }

    pub(super) async fn request_scene_state(
        &self,
        simulation_sender: &mpsc::Sender<SimulationTask>,
    ) -> Result<()> {
        simulation_sender
            .send(SimulationTask::RequestSceneState {
                sender: self.websocket_sender.clone(),
            })
            .await
            .wrap_err("channel closed")
    }

    pub(super) async fn request_scene_description(
        &self,
        simulation_sender: &mpsc::Sender<SimulationTask>,
    ) -> Result<()> {
        simulation_sender
            .send(SimulationTask::RequestSceneDescription {
                sender: self.websocket_sender.clone(),
            })
            .await
            .wrap_err("channel closed")
    }

    pub(super) async fn apply_low_command(
        &self,
        simulation_sender: &mpsc::Sender<SimulationTask>,
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        let send_task = async || -> Result<()> {
            simulation_sender
                .send(SimulationTask::ApplyLowCommand { receiver: rx })
                .await
                .wrap_err("channel closed")
        };

        let receive_low_command = async || -> Result<()> {
            let low_command = self
                .low_command_receiver
                .lock()
                .await
                .recv()
                .await
                .wrap_err("stream closed")?;
            tx.send(low_command).map_err(|_| eyre!("receiver dropped"))
        };

        tokio::try_join!(send_task(), receive_low_command(),)?;
        Ok(())
    }
}
