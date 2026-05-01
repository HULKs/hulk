use std::{
    fmt::Debug,
    future::ready,
    time::{Duration, SystemTime},
};

use booster::{LowCommand, LowState};
use pyo3::{Bound, Py, PyAny, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
use pyo3_async_runtimes::tokio::future_into_py;
use ros_z_msgs::{
    builtin_interfaces::Time,
    sensor_msgs::{CameraInfo, Image, RegionOfInterest},
    std_msgs::Header,
};
use ros2::pyo3_compat::sensor_msgs::{
    camera_info::CameraInfo as Ros2CameraInfo, image::Image as Ros2Image,
};
use simulation_message::{ConnectionInfo, SceneDescription, SceneUpdate, TaskName};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

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
        }
    }
}

#[derive(Debug)]
pub enum SimulationTask {
    StepSimulation {
        sender: oneshot::Sender<SystemTime>,
    },
    Reset,
    RequestLowState {
        sender: mpsc::Sender<SimulationData>,
    },
    RequestImage {
        sender: mpsc::Sender<SimulationData>,
    },
    RequestCameraInfo {
        sender: mpsc::Sender<SimulationData>,
    },
    ApplyLowCommand {
        receiver: oneshot::Receiver<LowCommand>,
    },
    Invalid,
    RequestSceneDescription {
        sender: mpsc::Sender<SimulationData>,
    },
    RequestSceneState {
        sender: mpsc::Sender<SimulationData>,
    },
}

pub enum SimulationData {
    SceneDescription {
        time: SystemTime,
        data: SceneDescription,
    },
    SceneState {
        time: SystemTime,
        data: SceneUpdate,
    },
    LowState {
        time: SystemTime,
        data: Box<LowState>,
    },
    Image {
        time: SystemTime,
        data: Box<Image>,
    },
    CameraInfo {
        time: SystemTime,
        data: Box<CameraInfo>,
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
            SimulationTask::RequestLowState { .. } => TaskName::RequestLowState,
            SimulationTask::Invalid => TaskName::Invalid,
            SimulationTask::ApplyLowCommand { .. } => TaskName::ApplyLowCommand,
            SimulationTask::RequestSceneDescription { .. } => TaskName::RequestSceneDescription,
            SimulationTask::RequestSceneState { .. } => TaskName::RequestSceneState,
            SimulationTask::RequestImage { .. } => TaskName::RequestImage,
            SimulationTask::RequestCameraInfo { .. } => TaskName::RequestCameraInfo,
        }
    }

    pub fn receive<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task = std::mem::replace(&mut self.task, SimulationTask::Invalid);
        match task {
            SimulationTask::ApplyLowCommand { receiver } => future_into_py(py, async move {
                // Channel may be closed if websocket closes.
                Ok(receiver.await.ok())
            }),
            _ => Err(PyValueError::new_err("no implementation for receive")),
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
            SimulationTask::RequestLowState { sender } => {
                let data = Box::new(response.extract(py)?);
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender.send(SimulationData::LowState { time, data }).await;
                    Ok(())
                })
            }
            SimulationTask::RequestImage { sender } => {
                let data = image_from_ros2(response.extract::<Ros2Image>(py)?);
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
            SimulationTask::RequestCameraInfo { sender } => {
                let data = camera_info_from_ros2(response.extract::<Ros2CameraInfo>(py)?);
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender
                        .send(SimulationData::CameraInfo {
                            time,
                            data: Box::new(data),
                        })
                        .await;
                    Ok(())
                })
            }
            SimulationTask::RequestSceneDescription { sender } => {
                let data = response.extract(py)?;
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender
                        .send(SimulationData::SceneDescription { time, data })
                        .await;
                    Ok(())
                })
            }
            SimulationTask::RequestSceneState { sender } => {
                let data = response.extract(py)?;
                future_into_py(py, async move {
                    // Channel may be closed if websocket disconnects
                    let _ = sender.send(SimulationData::SceneState { time, data }).await;
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

fn image_from_ros2(image: Ros2Image) -> Image {
    Image {
        header: header_from_ros2(image.header),
        height: image.height,
        width: image.width,
        encoding: image.encoding,
        is_bigendian: image.is_bigendian,
        step: image.step,
        data: image.data.into(),
    }
}

fn camera_info_from_ros2(camera_info: Ros2CameraInfo) -> CameraInfo {
    CameraInfo {
        header: header_from_ros2(camera_info.header),
        height: camera_info.height,
        width: camera_info.width,
        distortion_model: camera_info.distortion_model,
        d: camera_info.d,
        k: camera_info.k,
        r: camera_info.r,
        p: camera_info.p,
        binning_x: camera_info.binning_x,
        binning_y: camera_info.binning_y,
        roi: RegionOfInterest {
            x_offset: camera_info.roi.x_offset,
            y_offset: camera_info.roi.y_offset,
            height: camera_info.roi.height,
            width: camera_info.roi.width,
            do_rectify: camera_info.roi.do_rectify,
        },
    }
}

fn header_from_ros2(header: ros2::pyo3_compat::std_msgs::header::Header) -> Header {
    Header {
        stamp: Time {
            sec: header.stamp.sec,
            nanosec: header.stamp.nanosec,
        },
        frame_id: header.frame_id,
    }
}
