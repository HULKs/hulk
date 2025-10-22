use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use pyo3::pyclass;
use simulation_message::{ClientMessageKind, ServerMessageKind, SimulationMessage};
use tokio::{
    select,
    sync::{
        broadcast::{error::SendError, Receiver, Sender},
        Semaphore,
    },
};

pub struct SimulationState {
    pub is_connected: Arc<Semaphore>,
    pub to_simulation: Sender<ClientMessageKind>,
    pub from_simulation: Sender<SimulationMessage<ServerMessageKind>>,
    pub simulation_control: Sender<ServerCommand>,
}

#[pyclass(frozen, eq)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerCommand {
    Reset,
    RequestLowState,
    RequestRGBDSensors,
}

pub fn setup() -> (Router, Arc<SimulationState>) {
    let from_simulation = Sender::new(4);
    let to_simulation = Sender::new(4);
    let simulation_control = Sender::new(4);

    let state = Arc::new(SimulationState {
        is_connected: Arc::new(Semaphore::new(1)),
        to_simulation,
        from_simulation,
        simulation_control,
    });

    let router = Router::new()
        .route("/subscribe", get(ws_connection))
        .route("/reset", post(reset))
        .layer(Extension(state.clone()));

    (router, state)
}

async fn reset(Extension(state): Extension<Arc<SimulationState>>) -> impl IntoResponse {
    match state.simulation_control.send(ServerCommand::Reset) {
        Ok(_) => StatusCode::OK,
        Err(SendError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn ws_connection(
    Extension(state): Extension<Arc<SimulationState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let permit = match state.is_connected.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            log::warn!("someone is already connected, rejecting new connection");
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
    };

    let to_simulation = state.to_simulation.clone();
    let from_simulation = state.from_simulation.subscribe();

    ws.on_upgrade(async move |socket| {
        // keep the permit alive while the connection is active
        let _permit = permit;
        log::info!("Starting communication");
        handle_socket(socket, from_simulation, to_simulation).await;
    })
}

async fn handle_socket(
    mut socket: WebSocket,
    mut from_simulation: Receiver<SimulationMessage<ServerMessageKind>>,
    to_simulation: Sender<ClientMessageKind>,
) {
    loop {
        select! {
            simulator_message = from_simulation.recv() => {
                match simulator_message {
                    Ok(message) => {
                        let string = match serde_json::to_string(&message) {
                            Ok(string) => string,
                            Err(error) => {
                                log::error!("Failed to serialize message: {error}");
                                continue
                            }
                        };
                        match socket.send(Message::Text(string.into())).await {
                            Ok(()) => {},
                            Err(error) => {
                                log::error!("Failed to send into websocket, closing connection: {error}");
                                return
                            }
                        }
                    },
                    Err(error) => log::error!("{error}")
                }
            },
            interface_message = socket.recv() => {
                match interface_message {
                    Some(Ok(Message::Text(message))) => {
                        let message = match serde_json::from_str(message.as_str()) {
                            Ok(message) => message,
                            Err(error) => {
                                log::error!("Failed to deserialize: {error}");
                                continue;
                            }
                        };
                        if let Err(error) = to_simulation.send(message) {
                            log::error!("Failed to send message: {error}")
                        }
                    },
                    Some(Ok(Message::Binary(_))) => {
                        log::info!("Got unsupported binary message");
                    },
                    Some(Ok(_)) => {},
                    Some(Err(error)) => {
                        log::error!("{error}")
                    },
                    None => {
                        return
                    }
                }
            }
        }
    }
}
