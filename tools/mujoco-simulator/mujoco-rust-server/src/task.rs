use booster::{LowCommand, LowState};
use pyo3::{pyclass, pymethods, Py, PyAny, PyResult, Python};
use tokio::sync::oneshot;
use zed::RGBDSensors;

#[pyclass]
#[derive(Debug)]
pub struct ControllerTask {
    #[pyo3(get)]
    pub name: TaskName,
    state: TaskState,
}

#[pymethods]
impl ControllerTask {
    pub fn respond(&mut self, py: Python, response: Py<PyAny>) -> PyResult<()> {
        // Hack because pyo3 does not allow taking ownership of self.state directly
        let state = std::mem::replace(&mut self.state, TaskState::Done);
        match state {
            TaskState::RequestLowState { response: sender } => {
                let low_state: LowState = response.extract(py)?;
                sender.send(low_state).map_err(|_| {
                    pyo3::exceptions::PyRuntimeError::new_err("Failed to send LowState response")
                })
            }
            TaskState::RequestRGBDSensors { response: sender } => {
                let sensors: RGBDSensors = response.extract(py)?;
                sender.send(sensors).map_err(|_| {
                    pyo3::exceptions::PyRuntimeError::new_err("Failed to send RGBDSensors response")
                })
            }
            _ => Ok(()),
        }
    }
}

impl From<TaskState> for ControllerTask {
    fn from(state: TaskState) -> Self {
        let name = match &state {
            TaskState::ApplyLowCommand { .. } => TaskName::ApplyLowCommand,
            TaskState::RequestLowState { .. } => TaskName::RequestLowState,
            TaskState::RequestRGBDSensors { .. } => TaskName::RequestRGBDSensors,
            TaskState::StepSimulation => TaskName::StepSimulation,
            TaskState::Reset => TaskName::Reset,
            TaskState::Done => panic!("cannot create ControllerTask from Done state"),
        };
        Self { name, state }
    }
}

#[pyclass(frozen)]
#[derive(Copy, Clone, Debug)]
pub enum TaskName {
    ApplyLowCommand,
    RequestLowState,
    RequestRGBDSensors,
    StepSimulation,
    Reset,
}

#[derive(Debug)]
pub enum TaskState {
    Done,
    ApplyLowCommand {
        command: oneshot::Receiver<LowCommand>,
    },
    RequestLowState {
        response: oneshot::Sender<LowState>,
    },
    RequestRGBDSensors {
        response: oneshot::Sender<RGBDSensors>,
    },
    StepSimulation,
    Reset,
}
