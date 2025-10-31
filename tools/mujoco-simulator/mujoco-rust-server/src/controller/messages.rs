use std::{
    fmt::Debug,
    future::ready,
    time::{Duration, SystemTime},
};

use booster::{LowCommand, LowState};
use bytes::Bytes;
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Bound, Py, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use zed::RGBDSensors;

use crate::controller::handle::ConnectionHandle;

pub enum ControlCommand {
    Connect {
        sender: oneshot::Sender<ConnectionHandle>,
    },
    Disconnect {
        id: Uuid,
        sender: oneshot::Sender<()>,
    },
    Reset,
    Play,
    Pause,
}

impl Debug for ControlCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlCommand::Connect { .. } => f.debug_struct("Connect").finish(),
            ControlCommand::Disconnect { .. } => f.debug_struct("Connect").finish(),
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
    SceneState(Bytes),
    LowState { time: SystemTime, data: LowState },
    Image { time: SystemTime, data: Box<RGBDSensors> },
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
        }
    }

    pub fn receive<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task = std::mem::replace(&mut self.task, SimulationTask::Invalid);
        match task {
            SimulationTask::ApplyLowCommand { receiver } => future_into_py(py, async move {
                receiver
                    .await
                    .map_err(|_| PyValueError::new_err("failed to receive LowCommand"))
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
        log::info!("Responding to task: {:?}", self.kind());

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
                    sender
                        .send(SimulationData::LowState { time, data })
                        .await
                        .map_err(|_| PyValueError::new_err("failed to send LowState update"))
                })
            }
            SimulationTask::SceneDescription { sender } => {
                let data: Vec<u8> = response.extract(py)?;
                future_into_py(py, async move {
                    sender
                        .send(SimulationData::SceneDescription(data.into()))
                        .await
                        .map_err(|_| PyValueError::new_err("failed to send LowState update"))
                })
            }
            SimulationTask::SceneState { sender } => {
                let data: Vec<u8> = response.extract(py)?;
                future_into_py(py, async move {
                    sender
                        .send(SimulationData::SceneState(data.into()))
                        .await
                        .map_err(|_| PyValueError::new_err("failed to send LowState update"))
                })
            }
            SimulationTask::ApplyLowCommand { .. } => Err(PyValueError::new_err(
                "no need to call respond(..) for ApplyLowCommand",
            )),
            SimulationTask::Invalid => Err(PyValueError::new_err("encountered Invalid task")),
        }
    }
}

#[pyclass(frozen, eq)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TaskName {
    ApplyLowCommand,
    RequestLowState,
    RequestRGBDSensors,
    StepSimulation,
    Reset,
    Invalid,
    RequestSceneState,
    RequestSceneDescription,
}
