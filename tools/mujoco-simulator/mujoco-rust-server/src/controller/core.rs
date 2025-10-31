use std::ops::Range;
use std::sync::Arc;
use std::{collections::HashMap, time::SystemTime};

use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use simulation_message::{ConnectionInfo, OnceTask, PeriodicalTask};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::connection::Connection;
use super::handle::{ConnectionHandle, ControllerHandle};
use super::messages::{ControlCommand, SimulationTask};

fn log_result_info<T, E: std::fmt::Display>(result: Result<T, E>) {
    if let Err(err) = result {
        log::info!("{err}")
    }
}

pub struct Controller {
    simulation_time: SystemTime,
    paused: watch::Sender<bool>,

    connections: HashMap<Uuid, Connection>,
    simulation_task_sender: mpsc::Sender<SimulationTask>,
    control_sender: mpsc::Sender<ControlCommand>,
    control_receiver: mpsc::Receiver<ControlCommand>,
}

impl Controller {
    pub fn new(simulation_task_sender: mpsc::Sender<SimulationTask>) -> Self {
        let (control_sender, control_receiver) = mpsc::channel(16);

        Controller {
            simulation_time: SystemTime::UNIX_EPOCH,
            paused: watch::Sender::new(true),
            connections: HashMap::new(),
            simulation_task_sender,
            control_sender,
            control_receiver,
        }
    }

    pub fn start(self, cancellation_token: CancellationToken) -> ControllerHandle {
        let handle = ControllerHandle {
            sender: self.control_sender.clone(),
        };
        tokio::spawn(cancellation_token.run_until_cancelled_owned(async move {
            if let Err(error) = self.start_worker().await {
                log::error!("controller stopped unexpectedly: {error}")
            }
        }));
        handle
    }

    async fn create_connection(&mut self, connection_info: ConnectionInfo) -> ConnectionHandle {
        let id = Uuid::new_v4();
        let (low_command_sender, low_command_receiver) = mpsc::channel(4);
        let (websocket_sender, websocket_receiver) = mpsc::channel(4);

        let connection = Connection {
            low_command_receiver: Arc::new(Mutex::new(low_command_receiver)),
            websocket_sender,
            connection_info: Arc::new(connection_info),
        };
        if self.connections.is_empty() {
            log_result_info(self.paused.send(false));
        }
        for task in connection.initial_tasks() {
            let result = match task {
                OnceTask::RequestSceneDescription => {
                    connection
                        .request_scene_description(&self.simulation_task_sender)
                        .await
                }
                OnceTask::Reset => self
                    .simulation_task_sender
                    .send(SimulationTask::Reset)
                    .await
                    .wrap_err("failed to send Reset"),
            };
            log_result_info(result);
        }

        self.connections.insert(id, connection);
        ConnectionHandle {
            id,
            control_sender: self.control_sender.clone(),
            low_command_sender,
            websocket_receiver,
        }
    }

    async fn handle_control_message(&mut self, command: ControlCommand) {
        log::info!("received control message: {command:?}");
        match command {
            ControlCommand::Connect {
                sender,
                connection_info,
            } => {
                let connection = self.create_connection(connection_info).await;
                let result = sender
                    .send(connection)
                    .map_err(|_| eyre!("failed to return ConnectionHandle"));
                log_result_info(result);
            }
            ControlCommand::Disconnect { sender, id } => {
                self.connections.remove(&id);
                let result = sender
                    .send(())
                    .map_err(|_| eyre!("failed to reply to Disconnect"));
                log_result_info(result);
            }
            ControlCommand::Reset => {
                let result = self
                    .simulation_task_sender
                    .send(SimulationTask::Reset)
                    .await
                    .wrap_err("failed to send Reset");
                log_result_info(result);
            }
            ControlCommand::Play => log_result_info(self.paused.send(false)),
            ControlCommand::Pause => log_result_info(self.paused.send(true)),
        }
    }

    async fn start_worker(mut self) -> Result<()> {
        let range = self.simulation_time..self.simulation_time;
        let mut tasks = Box::pin(self.task_stream(range).join_all());

        loop {
            tokio::select! {
                Some(command) = self.control_receiver.recv() => {
                    self.handle_control_message(command).await;
                },
                _ = &mut tasks => {
                    let range = self.perform_simulation_step().await?;
                    tasks = Box::pin(self.task_stream(range).join_all())
                },
            }
        }
    }

    async fn perform_simulation_step(&mut self) -> Result<Range<SystemTime>> {
        let earlier = self.simulation_time;
        let (tx, rx) = oneshot::channel();
        log_result_info(
            self.simulation_task_sender
                .send(SimulationTask::StepSimulation { sender: tx })
                .await,
        );
        let now = rx.await.wrap_err("channel closed")?;
        self.simulation_time = now;
        Ok(earlier..now)
    }

    /// Returns a stream of simulation tasks for the current time of all connections.
    fn task_stream(&self, range: Range<SystemTime>) -> JoinSet<()> {
        let mut tasks = JoinSet::new();
        let mut paused = self.paused.subscribe();
        tasks.spawn(async move {
            log_result_info(paused.wait_for(|paused| !paused).await);
        });

        for connection in self.connections.values().cloned() {
            let sender = self.simulation_task_sender.clone();
            let range = range.clone();
            tasks.spawn(async move {
                let sender = sender;
                for task in connection.due_tasks(range) {
                    let result = match task {
                        PeriodicalTask::ApplyLowCommand => {
                            connection.apply_low_command(&sender).await
                        }
                        PeriodicalTask::RequestLowState => {
                            connection.request_low_state(&sender).await
                        }
                        PeriodicalTask::RequestRGBDSensors => {
                            connection.request_rgbd_sensors(&sender).await
                        }
                        PeriodicalTask::RequestSceneState => {
                            connection.request_scene_state(&sender).await
                        }
                        PeriodicalTask::RequestSceneDescription => {
                            connection.request_scene_description(&sender).await
                        }
                    };
                    log_result_info(result);
                }
            });
        }
        tasks
    }
}
