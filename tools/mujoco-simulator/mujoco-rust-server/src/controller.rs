use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use booster::{LowCommand, LowState};
use tokio::{
    select,
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use zed::RGBDSensors;

use crate::{
    state_machine::PeriodicalTask,
    task::{TaskName, TaskState},
};

pub struct Controller {
    now: SystemTime,
    connections: HashMap<Uuid, ConnectionState>,
    task_sender: Sender<TaskState>,

    command_sender: Sender<ControllerCommand>,
    command_receiver: Receiver<ControllerCommand>,
}

pub struct ConnectionState {
    sender: Sender<ControllerData>,
    tasks: Vec<PeriodicalTask<TaskName>>,
}

pub enum ControllerData {
    LowState(oneshot::Receiver<LowState>),
    RGBDSensors(oneshot::Receiver<RGBDSensors>),
    GetLowCommand(oneshot::Sender<LowCommand>),
}

#[derive(Clone)]
pub struct ControllerHandle {
    sender: Sender<ControllerCommand>,
}

impl ControllerHandle {
    async fn send_to_controller(&self, command: ControllerCommand) {
        self.sender
            .send(command)
            .await
            .expect("controller actor is dead")
    }

    pub async fn add_connection(&self, id: Uuid) -> Receiver<ControllerData> {
        let (tx, rx) = oneshot::channel();
        self.send_to_controller(ControllerCommand::AddConnection { id, response: tx })
            .await;
        rx.await.expect("controller actor failed to respond")
    }

    pub async fn remove_connection(&self, id: Uuid) -> bool {
        let (tx, rx) = oneshot::channel();
        self.send_to_controller(ControllerCommand::RemoveConnection { id, response: tx })
            .await;
        rx.await.expect("controller actor failed to respond")
    }

    pub async fn request_low_state(&self, connection_id: Uuid) {
        self.send_to_controller(ControllerCommand::RequestLowState { connection_id })
            .await;
    }

    pub async fn request_rgbd_sensors(&self, connection_id: Uuid) {
        self.send_to_controller(ControllerCommand::RequestRGBDSensors { connection_id })
            .await;
    }

    pub async fn advance_time(&self, now: SystemTime) {
        self.send_to_controller(ControllerCommand::AdvanceTime { now })
            .await;
    }

    pub async fn reset(&self) {
        self.send_to_controller(ControllerCommand::Reset).await;
    }
}

#[derive(Debug)]
pub enum ControllerCommand {
    AdvanceTime {
        now: SystemTime,
    },
    AddConnection {
        id: Uuid,
        response: oneshot::Sender<Receiver<ControllerData>>,
    },
    RemoveConnection {
        id: Uuid,
        response: oneshot::Sender<bool>,
    },
    RequestLowState {
        connection_id: Uuid,
    },
    RequestRGBDSensors {
        connection_id: Uuid,
    },
    Reset,
}

impl Controller {
    pub fn new(task_sender: Sender<TaskState>) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(8);
        Self {
            connections: HashMap::new(),
            now: SystemTime::UNIX_EPOCH,
            task_sender,
            command_sender,
            command_receiver,
        }
    }

    pub fn handle(&self) -> ControllerHandle {
        ControllerHandle {
            sender: self.command_sender.clone(),
        }
    }

    pub async fn start(self, token: CancellationToken) {
        log::info!("Starting controller");
        token.run_until_cancelled_owned(self.start_worker()).await;
        log::info!("Controller stopped");
    }

    async fn start_worker(mut self) {
        while let Some(message) = self.command_receiver.recv().await {
            match message {
                ControllerCommand::AdvanceTime { now } => {
                    if self.now == now {
                        continue;
                    }
                    log::info!(
                        "Advancing time to {}ms",
                        now.duration_since(SystemTime::UNIX_EPOCH)
                            .expect("time ran backwards")
                            .as_millis()
                    );
                }
                ControllerCommand::AddConnection { id, response } => {
                    let tasks = vec![];
                    let (sender, receiver) = mpsc::channel(8);
                    self.connections
                        .insert(id, ConnectionState { sender, tasks });
                    response.send(receiver).expect("failed to add connection");
                }
                ControllerCommand::RemoveConnection { id, response } => {
                    let existed = self.connections.remove(&id).is_some();
                    response.send(existed).expect("failed to remove connection");
                }
                ControllerCommand::RequestLowState { connection_id } => {
                    let sender = self
                        .connections
                        .get(&connection_id)
                        .expect("unknown connection id")
                        .sender
                        .clone();
                    let task_sender = self.task_sender.clone();
                    tokio::spawn(async move {
                        let (tx, rx) = oneshot::channel();
                        task_sender
                            .send(TaskState::RequestLowState { response: tx })
                            .await
                            .expect("failed to send RequestLowState task");
                        let _ = sender.send(ControllerData::LowState(rx)).await;
                    });
                }
                ControllerCommand::RequestRGBDSensors { connection_id } => {
                    let sender = self
                        .connections
                        .get(&connection_id)
                        .expect("unknown connection id")
                        .sender
                        .clone();
                    let task_sender = self.task_sender.clone();
                    tokio::spawn(async move {
                        let (tx, rx) = oneshot::channel();
                        task_sender
                            .send(TaskState::RequestRGBDSensors { response: tx })
                            .await
                            .expect("failed to send RequestRGBDSensors task");
                        let _ = sender.send(ControllerData::RGBDSensors(rx)).await;
                    });
                }
                ControllerCommand::Reset => todo!(),
            }
        }
        // loop {

        //     let received_message =
        //         timeout(Duration::from_millis(100), self.command_receiver.recv()).await;
        //     log::info!("controller cycle: {:?}", received_message);
        // }
    }

    // pub fn add_connection(&mut self, id: Uuid) -> Receiver<ControllerData> {
    //     let (sender, receiver) = channel(8);
    //     let tasks = vec![
    //         PeriodicalTask::new(Duration::from_millis(10), TaskName::RequestLowState),
    //         PeriodicalTask::new(Duration::from_millis(100), TaskName::RequestRGBDSensors),
    //         PeriodicalTask::new(Duration::from_millis(10), TaskName::ApplyLowCommand),
    //     ];
    //     self.connections
    //         .insert(id, ConnectionState { sender, tasks });
    //     receiver
    // }

    // pub fn remove_connection(&mut self, id: Uuid) -> bool {
    //     self.connections.remove(&id).is_some()
    // }

    // pub async fn reset(&self) -> Result<(), SendError<TaskState>> {
    //     self.task_sender.send(TaskState::Reset).await
    // }

    // pub async fn request_low_state(&self, connection_id: Uuid) -> Result<(), SendError<TaskState>> {
    //     let (sender, receiver) = oneshot::channel();
    //     let connection_sender = self.connections.get(&connection_id).unwrap().sender.clone();
    //     connection_sender
    //         .send(ControllerData::LowState(receiver))
    //         .await
    //         .unwrap();

    //     self.task_sender
    //         .send(TaskState::RequestLowState { response: sender })
    //         .await
    // }

    // pub async fn request_rgbd_sensors(
    //     &self,
    //     connection_id: Uuid,
    // ) -> Result<(), SendError<TaskState>> {
    //     let (sender, receiver) = oneshot::channel();
    //     let connection_sender = self.connections.get(&connection_id).unwrap().sender.clone();
    //     connection_sender
    //         .send(ControllerData::RGBDSensors(receiver))
    //         .await
    //         .unwrap();

    //     self.task_sender
    //         .send(TaskState::RequestRGBDSensors { response: sender })
    //         .await
    // }

    // pub fn advance_time(&mut self, now: SystemTime) {
    //     if self.now == now {
    //         return;
    //     }
    //     self.now = now;

    //     let mut join_set = JoinSet::new();
    //     let duration = now
    //         .duration_since(SystemTime::UNIX_EPOCH)
    //         .expect("time ran backwards");

    //     for connection in self.connections.values_mut() {
    //         for task in connection.tasks.iter_mut() {
    //             let Some(task_name) = task.task(now) else {
    //                 continue;
    //             };
    //             log::info!(
    //                 "{}ms: Scheduling task {:?}",
    //                 duration.as_millis(),
    //                 task_name
    //             );
    //             let connection_sender = connection.sender.clone();
    //             let task_state = match task_name {
    //                 TaskName::ApplyLowCommand => {
    //                     let (sender, receiver) = oneshot::channel();
    //                     join_set.spawn(async move {
    //                         connection_sender
    //                             .send(ControllerData::GetLowCommand(sender))
    //                             .await
    //                             .unwrap();
    //                     });
    //                     TaskState::ApplyLowCommand { command: receiver }
    //                 }
    //                 TaskName::RequestLowState => {
    //                     let (sender, receiver) = oneshot::channel();
    //                     join_set.spawn(async move {
    //                         connection_sender
    //                             .send(ControllerData::LowState(receiver))
    //                             .await
    //                             .unwrap();
    //                     });
    //                     TaskState::RequestLowState { response: sender }
    //                 }
    //                 TaskName::RequestRGBDSensors => {
    //                     let (sender, receiver) = oneshot::channel();
    //                     join_set.spawn(async move {
    //                         connection_sender
    //                             .send(ControllerData::RGBDSensors(receiver))
    //                             .await
    //                             .unwrap();
    //                     });
    //                     TaskState::RequestRGBDSensors { response: sender }
    //                 }
    //                 TaskName::StepSimulation => TaskState::StepSimulation,
    //                 TaskName::Reset => TaskState::Reset,
    //             };

    //             let task_sender = self.task_sender.clone();
    //             join_set.spawn(async move {
    //                 if let Err(error) = task_sender.send(task_state).await {
    //                     log::error!("Failed to send task: {}", error);
    //                 }
    //             });
    //         }
    //     }

    //     tokio::spawn(join_set.join_all());
    // }
}
