mod scene;
mod simulation;

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{routing::get, Router};
use bytes::Bytes;
use pyo3::{exceptions::PyValueError, pyclass, pymethods, Py, PyResult, Python};
use tokio::{
    net::TcpListener,
    runtime::Runtime,
    select,
    sync::{
        broadcast::{error::TryRecvError, Receiver, Sender},
        Semaphore,
    },
};
use tokio_util::sync::CancellationToken;

use booster::{LowCommand, LowState};
use simulation_message::{ClientMessageKind, ServerMessageKind, SimulationMessage};
use tower_http::cors::{Any, CorsLayer};
use zed::RGBDSensors;

#[pyclass]
pub struct SimulationServer {
    runtime: Runtime,
    cancel_token: CancellationToken,

    simulation: SimulationSideChannels,
}

struct SimulationSideChannels {
    permits: Arc<Semaphore>,
    simulation_control: Receiver<simulation::ServerCommand>,
    message_receiver: Receiver<ClientMessageKind>,
    message_sender: Sender<SimulationMessage<ServerMessageKind>>,
    scene_state: Arc<scene::SceneState>,
}

#[pymethods]
impl SimulationServer {
    #[new]
    pub fn start(bind_address: &str) -> PyResult<Self> {
        let runtime = Runtime::new()?;
        let cancel_token = CancellationToken::new();

        let (scene_router, scene_state) = scene::setup();
        let (simulation_router, simulation_state) = simulation::setup();

        let this = SimulationServer {
            runtime,
            cancel_token,
            simulation: SimulationSideChannels {
                permits: simulation_state.is_connected.clone(),
                simulation_control: simulation_state.simulation_control.subscribe(),
                message_receiver: simulation_state.to_simulation.subscribe(),
                message_sender: simulation_state.from_simulation.clone(),
                scene_state: scene_state.clone(),
            },
        };

        let bind_address = bind_address.to_string();
        let token = this.cancel_token.clone();
        this.runtime.spawn(async move {
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

            select! {
                _ = token.cancelled() => {
                    log::info!("Shutdown signal received, stopping server.");
                }
                result = axum::serve(listener, app) => {
                    match result {
                        Ok(_) => log::info!("Server stopped"),
                        Err(e) => log::error!("Error serving the application: {}", e),
                    }
                }
            };
        });

        Ok(this)
    }

    pub fn send_low_state(&self, simulation_time: f32, low_state: Py<LowState>) -> PyResult<()> {
        // ignore the error, as it just means there are no receivers

        let _ = self.simulation.message_sender.send(SimulationMessage {
            time: SystemTime::UNIX_EPOCH + Duration::from_secs_f32(simulation_time),
            payload: ServerMessageKind::LowState(low_state.get().clone()),
        });
        Ok(())
    }

    pub fn is_client_connected(&self) -> bool {
        self.simulation.permits.available_permits() == 0
    }

    pub fn register_scene(&self, scene: Vec<u8>) -> PyResult<()> {
        self.simulation
            .scene_state
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
        let _ = self
            .simulation
            .scene_state
            .scene_sender
            .send(scene_state.to_string());
        Ok(())
    }

    pub fn receive_low_command(&mut self) -> Option<LowCommand> {
        match self.simulation.message_receiver.try_recv() {
            Ok(ClientMessageKind::LowCommand(low_command)) => Some(low_command),
            Err(TryRecvError::Empty) => None,
            Err(error) => {
                log::error!("Failed to receive motor command: {error}");
                None
            }
        }
    }

    pub fn receive_low_command_blocking(&mut self, py: Python) -> PyResult<LowCommand> {
        let check_signals = async move || -> PyResult<()> {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                py.check_signals()?;
            }
        };

        let mut receive_low_command = async || match self.simulation.message_receiver.recv().await {
            Ok(ClientMessageKind::LowCommand(low_command)) => Ok(low_command),
            Err(error) => {
                log::error!("Failed to receive motor command: {error}");
                return Err(PyValueError::new_err("Failed to receive motor command"));
            }
        };

        self.runtime.block_on(async {
            select! {
                _ = check_signals() => Err(PyValueError::new_err("Interrupted by signal")),
                result = receive_low_command() => return result,
            }
        })
    }

    pub fn receive_simulation_command(&mut self) -> Option<simulation::ServerCommand> {
        match self.simulation.simulation_control.try_recv() {
            Ok(command) => Some(command),
            Err(TryRecvError::Empty) => None,
            Err(error) => {
                log::error!("Failed to receive simulation command: {error}");
                None
            }
        }
    }

    pub fn send_camera_frame(
        &self,
        simulation_time: f32,
        rgbd_sensors: Py<RGBDSensors>,
    ) -> PyResult<()> {
        log::debug!("Sending frame");
        let _ = self.simulation.message_sender.send(SimulationMessage {
            time: SystemTime::UNIX_EPOCH + Duration::from_secs_f32(simulation_time),
            payload: ServerMessageKind::RGBDSensors(rgbd_sensors.get().clone()),
        });
        Ok(())
    }

    pub fn stop(&self) {
        log::info!("Stopping server");
        self.cancel_token.cancel();
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
