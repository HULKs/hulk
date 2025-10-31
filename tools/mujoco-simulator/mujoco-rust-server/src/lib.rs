mod controller;
mod simulation;

use std::{future::IntoFuture, sync::Arc};

use axum::{routing::get, Router};
use bytes::Bytes;
use pyo3::pymodule;
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Bound, PyAny, PyResult, Python};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    sync::{
        mpsc::{self, Receiver},
        Mutex,
    },
};
use tokio_util::sync::CancellationToken;

use tower_http::cors::{Any, CorsLayer};

use crate::controller::{Controller, PySimulationTask, SimulationTask};

#[pyclass]
pub struct SimulationServer {
    _runtime: Runtime,
    cancellation_token: CancellationToken,
    task_receiver: Arc<Mutex<Receiver<SimulationTask>>>,
}

#[pymethods]
impl SimulationServer {
    #[new]
    pub fn start(bind_address: &str) -> PyResult<Self> {
        pyo3_log::init();
        let runtime = Runtime::new()?;
        let _guard = runtime.enter();
        let cancellation_token = CancellationToken::new();

        let (task_sender, task_receiver) = mpsc::channel(16);
        let controller = Controller::new(task_sender);
        let handle = controller.start(cancellation_token.clone());

        let simulation_router = simulation::setup(handle.clone());

        let bind_address = bind_address.to_string();

        let token = cancellation_token.clone();
        tokio::spawn(async move {
            let cors_layer = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route("/health", get(health_check))
                .nest("/simulation", simulation_router)
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
            _runtime: runtime,
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
        future_into_py(py, async move {
            log::info!("TODO: check everything is shutdown");
            Ok(())
        })
    }
}

async fn health_check() -> &'static str {
    "OK"
}

#[pymodule(name = "mujoco_rust_server")]
mod python_module {
    #[pymodule_export]
    use crate::{controller::PySimulationTask, SimulationServer};

    #[pymodule_export]
    use simulation_message::TaskName;

    #[pymodule_export(name = "booster_types")]
    use booster::python_module as booster_types;

    #[pymodule_export(name = "zed_types")]
    use zed::python_module as zed_types;
}
