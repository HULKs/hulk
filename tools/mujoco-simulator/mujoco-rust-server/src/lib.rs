mod controller;
mod websocket;

use std::sync::Arc;

use pyo3::pymodule;
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Bound, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::task::JoinSet;
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
};
use tokio_util::sync::CancellationToken;

use crate::controller::{Controller, ControllerHandle, PySimulationTask, SimulationTask};

async fn start_tcp_listener(bind_address: String, handle: ControllerHandle) {
    let listener = match TcpListener::bind(bind_address).await {
        Ok(listener) => listener,
        Err(e) => {
            log::error!("Failed to bind TCP listener: {}", e);
            return;
        }
    };
    log::info!("Server listening on {}", listener.local_addr().unwrap());

    while let Ok((stream, _)) = listener.accept().await {
        let handle = handle.clone();
        tokio::spawn(async move {
            if let Err(error) = websocket::accept_websocket(stream, handle.clone()).await {
                log::info!("{error}")
            }
        });
    }
}

#[pyclass]
pub struct SimulationServer {
    _runtime: Runtime,
    cancellation_token: CancellationToken,
    task_receiver: Arc<Mutex<Receiver<SimulationTask>>>,
    tasks: JoinSet<Option<()>>,
}

#[pymethods]
impl SimulationServer {
    #[new]
    pub fn start(bind_address: String) -> PyResult<Self> {
        pyo3_log::init();
        let cancellation_token = CancellationToken::new();

        let (task_sender, task_receiver) = mpsc::channel(16);
        let controller = Controller::new(task_sender);
        let handle = controller.handle();

        let runtime = Runtime::new()?;
        let _guard = runtime.enter();

        let mut tasks = JoinSet::new();
        tasks.spawn(
            cancellation_token
                .clone()
                .run_until_cancelled_owned(controller.start()),
        );
        tasks.spawn(
            cancellation_token
                .clone()
                .run_until_cancelled_owned(start_tcp_listener(bind_address, handle)),
        );

        Ok(SimulationServer {
            _runtime: runtime,
            tasks,
            cancellation_token,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
        })
    }

    pub fn next_task<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = self.task_receiver.clone();
        future_into_py(py, async move {
            match receiver.lock().await.recv().await {
                Some(task) => Ok(PySimulationTask::from(task)),
                None => Err(PyValueError::new_err("Channel closed")),
            }
        })
    }

    pub fn stop<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        log::info!("Stopping server");
        self.cancellation_token.cancel();
        let tasks = std::mem::take(&mut self.tasks);

        future_into_py(py, async move {
            tasks.join_all().await;
            log::info!("Server stopped");
            Ok(())
        })
    }
}

#[pymodule(name = "mujoco_rust_server")]
mod python_module {
    #[pymodule_export]
    use crate::{controller::PySimulationTask, SimulationServer};

    #[pymodule_export]
    use simulation_message::{
        Body, BodyUpdate, Geom, Light, Material, PbrMaterial, SceneDescription, SceneMesh,
        SceneUpdate, TaskName, Texture,
    };

    #[pymodule_export(name = "booster_types")]
    use booster::python_module as booster_types;

    #[pymodule_export(name = "ros2_types")]
    use ros2::python_module as ros2_types;
}
