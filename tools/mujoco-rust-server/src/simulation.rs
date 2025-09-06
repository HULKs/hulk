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
    sync::broadcast::{error::SendError, Receiver, Sender},
};

pub struct SimulationState {
    pub to_simulation: Sender<ClientMessageKind>,
    pub from_simulation: Sender<SimulationMessage<ServerMessageKind>>,
    pub simulation_control: Sender<ServerCommand>,
    pub camera_stream: Sender<Vec<u8>>,
}

#[pyclass(frozen, eq)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerCommand {
    Reset,
}

pub fn setup() -> (Router, Arc<SimulationState>) {
    let from_simulation = Sender::new(8);
    let to_simulation = Sender::new(8);
    let simulation_control = Sender::new(8);
    let camera_stream = Sender::new(8);

    let state = Arc::new(SimulationState {
        to_simulation,
        from_simulation,
        simulation_control,
        camera_stream,
    });

    let router = Router::new()
        .route("/subscribe", get(ws_connection))
        .route("/reset", post(reset))
        .route("/camera", get(ws_camera))
        .layer(Extension(state.clone()));

    (router, state)
}

async fn reset(Extension(state): Extension<Arc<SimulationState>>) -> impl IntoResponse {
    match state.simulation_control.send(ServerCommand::Reset) {
        Ok(_) => StatusCode::OK,
        Err(SendError(_)) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn ws_camera(
    Extension(state): Extension<Arc<SimulationState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    log::info!("Got camera stream request");
    let mut receiver = state.camera_stream.subscribe();
    ws.on_upgrade(async move |mut socket| {
        while let Ok(packet) = receiver.recv().await {
            if let Err(error) = socket.send(Message::Binary(packet.into())).await {
                log::error!("Camera ws: {error}");
                break;
            }
        }
    })
}

async fn ws_connection(
    Extension(state): Extension<Arc<SimulationState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    log::info!("Got websocket request");
    let to_simulation = state.to_simulation.clone();
    let from_simulation = state.from_simulation.subscribe();

    ws.on_upgrade(async move |socket| {
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
