use std::{
    fmt::Debug,
    future::ready,
    time::{Duration, SystemTime},
};

use booster::{LowCommand, LowState};
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Bound, Py, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use simulation_message::{ConnectionInfo, TaskName};
use tokio::sync::{mpsc, oneshot};
use tokio_util::bytes::Bytes;
use uuid::Uuid;
use zed::RGBDSensors;

use super::handle::ConnectionHandle;

pub enum ControlCommand {
    Connect {
        sender: oneshot::Sender<ConnectionHandle>,
        connection_info: ConnectionInfo,
    },
    Disconnect {
        id: Uuid,
        sender: oneshot::Sender<()>,
    },
}

impl Debug for ControlCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlCommand::Connect { .. } => f.debug_struct("Connect").finish(),
            ControlCommand::Disconnect { .. } => f.debug_struct("Disconnect").finish(),
            ControlCommand::Reset => f.debug_struct("Reset").finish(),
            ControlCommand::Play => f.debug_struct("Play").finish(),
            ControlCommand::Pause => f.debug_struct("Pause").finish(),
        }
    }
}

#[derive(Debug)]
pub enum SimulationTask {
    StepSimulation {
        sender: oneshot::Sender<SystemTime>,
    },
    Reset,
    LowState {
        sender: mpsc::Sender<SimulationData>,
    },
    RGBDSensors {
        sender: mpsc::Sender<SimulationData>,
    },
    ApplyLowCommand {
        receiver: oneshot::Receiver<LowCommand>,
    },
    Invalid,
    SceneDescription {
        sender: mpsc::Sender<SimulationData>,
    },
    SceneState {
        sender: mpsc::Sender<SimulationData>,
    },
}

pub enum SimulationData {
    SceneDescription(Bytes),
    SceneState(String),
    LowState {
        time: SystemTime,
        data: LowState,
    },
    Image {
        time: SystemTime,
        data: Box<RGBDSensors>,
    },
}

#[pyclass]
pub struct PySimulationTask {
    task: SimulationTask,
}

impl From<SimulationTask> for PySimulationTask {
    fn from(task: SimulationTask) -> Self {
        Self { task }
    }
}

#[pymethods]
impl PySimulationTask {
    pub fn kind(&self) -> TaskName {
        match self.task {
            SimulationTask::Reset => TaskName::Reset,
            SimulationTask::StepSimulation { .. } => TaskName::StepSimulation,
            SimulationTask::LowState { .. } => TaskName::RequestLowState,
            SimulationTask::Invalid => TaskName::Invalid,
            SimulationTask::ApplyLowCommand { .. } => TaskName::ApplyLowCommand,
            SimulationTask::SceneDescription { .. } => TaskName::RequestSceneDescription,
            SimulationTask::SceneState { .. } => TaskName::RequestSceneState,
            SimulationTask::RGBDSensors { .. } => TaskName::RequestRGBDSensors,
        }
    }

    pub fn receive<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task = std::mem::replace(&mut self.task, SimulationTask::Invalid);
        match task {
            SimulationTask::ApplyLowCommand { receiver } => future_into_py(py, async move {
                // Channel may be closed if websocket closes.
                Ok(receiver.await.ok())
            }),
            _ => Err(PyValueError::new_err("no implentation for receive")),
        }
    }

    pub fn respond<'py>(
        &mut self,
        py: Python<'py>,
        time: f32,
        response: Py<PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs_f32(time);

        let task = std::mem::replace(&mut self.task, SimulationTask::Invalid);
        match task {
            SimulationTask::StepSimulation { sender } => {
                sender
                    .send(time)
                    .map_err(|_| PyValueError::new_err("failed to send SimulationStep"))?;
                future_into_py(py, ready(Ok(())))
            }
            SimulationTask::Reset => future_into_py(py, ready(Ok(()))),
            SimulationTask::LowState { sender } => {
                let data = response.extract(py)?;
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender.send(SimulationData::LowState { time, data }).await;
                    Ok(())
                })
            }
            SimulationTask::RGBDSensors { sender } => {
                let data = response.extract(py)?;
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender
                        .send(SimulationData::Image {
                            time,
                            data: Box::new(data),
                        })
                        .await;
                    Ok(())
                })
            }
            SimulationTask::SceneDescription { sender } => {
                let data: Vec<u8> = response.extract(py)?;
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender
                        .send(SimulationData::SceneDescription(data.into()))
                        .await;
                    Ok(())
                })
            }
            SimulationTask::SceneState { sender } => {
                let data = response.extract(py)?;
                future_into_py(py, async move {
                    let _ = sender.send(SimulationData::SceneState(data)).await;
                    Ok(())
                })
            }
            SimulationTask::ApplyLowCommand { .. } => Err(PyValueError::new_err(
                "no need to call respond(..) for ApplyLowCommand",
            )),
            SimulationTask::Invalid => Err(PyValueError::new_err("encountered Invalid task")),
        }
    }
}
