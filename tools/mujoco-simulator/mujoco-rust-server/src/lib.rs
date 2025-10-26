mod controller;
mod scene;
mod simulation;
mod state_machine;
mod task;

use std::{
    future::IntoFuture,
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{error_handling::future, routing::get, Router};
use bytes::Bytes;
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Bound, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
    task::JoinSet,
    time::{sleep, timeout},
};
use tokio_util::sync::CancellationToken;

use tower_http::cors::{Any, CorsLayer};

use crate::{
    controller::{Controller, ControllerHandle},
    task::{ControllerTask, TaskState},
};

#[pyclass]
pub struct SimulationServer {
    runtime: Runtime,
    cancellation_token: CancellationToken,
    scene_state: Arc<scene::SceneState>,
    task_receiver: Arc<Mutex<Receiver<TaskState>>>,
    controller_handle: ControllerHandle,

    tasks: JoinSet<()>,
}

#[pymethods]
impl SimulationServer {
    #[new]
    pub fn start(bind_address: &str) -> PyResult<Self> {
        let id = unsafe { libc::pthread_self() };
        log::info!("Thread id in next_task: {:?}", id);

        let runtime = Runtime::new()?;
        let _guard = runtime.enter();
        let cancellation_token = CancellationToken::new();

        let (task_sender, task_receiver) = mpsc::channel(16);
        let controller = Controller::new(task_sender);
        let handle = controller.handle();
        let mut tasks = JoinSet::new();

        tasks.spawn(controller.start(cancellation_token.clone()));

        let (scene_router, scene_state) = scene::setup();
        let simulation_router = simulation::setup(handle.clone());

        let bind_address = bind_address.to_string();

        let token = cancellation_token.clone();
        tasks.spawn(async move {
            let cors_layer = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route("/health", get(health_check))
                .nest("/simulation", simulation_router)
                .nest("/scene", scene_router)
                .layer(cors_layer);

            let listener = match TcpListener::bind(bind_address).await {
                Ok(listener) => listener,
                Err(e) => {
                    log::error!("Failed to bind TCP listener: {}", e);
                    return;
                }
            };

            log::info!("Server listening on {}", listener.local_addr().unwrap());
            token
                .run_until_cancelled_owned(axum::serve(listener, app).into_future())
                .await;
            log::info!("Server stopped");
        });

        Ok(SimulationServer {
            runtime,
            cancellation_token,
            scene_state,
            task_receiver: Arc::new(Mutex::new(task_receiver)),
            controller_handle: handle,
            tasks,
        })
    }

    pub fn next_task<'py>(
        &mut self,
        py: Python<'py>,
        simulation_time: f32,
    ) -> PyResult<Bound<'py, PyAny>> {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs_f32(simulation_time);
        let handle = self.controller_handle.clone();
        let receiver = self.task_receiver.clone();

        future_into_py(py, async move {
            handle.advance_time(now).await;
            match receiver.lock().await.recv().await {
                Some(task) => Ok(ControllerTask::from(task)),
                None => Err(PyValueError::new_err("Channel closed")),
            }
        })
    }

    pub fn example_async_task<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        future_into_py(py, async {
            sleep(Duration::from_secs(5)).await;
            log::info!("Async task wrapper completed.");
            Ok(())
        })
    }

    pub fn register_scene(&self, scene: Vec<u8>) -> PyResult<()> {
        self.scene_state
            .scene
            .set(Bytes::from(scene))
            .map_err(|_| {
                log::error!("Scene already set");
                PyValueError::new_err("Scene already set")
            })?;

        log::info!("Scene registered");
        Ok(())
    }

    pub fn update_scene_state(&self, scene_state: &str) -> PyResult<()> {
        // ignore the error, as it just means there are no receivers
        let _ = self.scene_state.scene_sender.send(scene_state.to_string());
        Ok(())
    }

    pub fn stop<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        log::info!("Stopping server");
        self.cancellation_token.cancel();
        let mut tasks = std::mem::take(&mut self.tasks);
        future_into_py(py, async move {
            tasks.shutdown().await;
            Ok(())
        })
    }
}

async fn health_check() -> &'static str {
    "OK"
}

mod python_bindings {
    use pyo3::{prelude::*, py_run, pymodule};

    #[pymodule(name = "mujoco_rust_server")]
    fn extension(m: &Bound<'_, PyModule>) -> PyResult<()> {
        pyo3_log::init();
        m.add_class::<crate::SimulationServer>()?;
        m.add_class::<crate::simulation::ServerCommand>()?;
        let submodule = PyModule::new(m.py(), "booster_types")?;
        booster::python_bindings::extension(&submodule)?;
        py_run!(
            m.py(),
            submodule,
            "import sys; sys.modules['mujoco_rust_server.booster_types'] = submodule"
        );
        m.add_submodule(&submodule)?;

        let submodule = PyModule::new(m.py(), "zed_types")?;
        zed::python_bindings::extension(&submodule)?;
        py_run!(
            m.py(),
            submodule,
            "import sys; sys.modules['mujoco_rust_server.zed_types'] = submodule"
        );
        m.add_submodule(&submodule)?;

        Ok(())
    }
}
