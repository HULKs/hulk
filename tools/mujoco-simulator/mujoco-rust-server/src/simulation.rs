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
use booster::{LowCommand, LowState};
use pyo3::pyclass;
use serde::Serialize;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;
use zed::RGBDSensors;

use crate::controller::{ControllerData, ControllerHandle};

#[pyclass(frozen)]
#[derive(Clone, Debug)]
pub enum ServerCommand {
    Reset(),
    RequestLowState(),
    RequestRGBDSensors(),
    ApplyLowCommand(LowCommand),
}

#[pyclass(frozen)]
#[derive(Clone, Debug)]
pub enum SimulationResponse {
    ResponseLowState(LowState),
    ResponseRGBDSensors(RGBDSensors),
}

pub fn setup(handle: ControllerHandle) -> Router {
    Router::new()
        .route("/subscribe", get(ws_connection))
        .route("/reset", post(reset))
        .layer(Extension(handle))
}

async fn reset(Extension(handle): Extension<ControllerHandle>) -> impl IntoResponse {
    handle.reset().await;
    StatusCode::OK
}

async fn ws_connection(
    Extension(handle): Extension<ControllerHandle>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let connection_id = Uuid::new_v4();
    log::info!("Got new websocket request (id={connection_id})");

    ws.on_upgrade(async move |socket| {
        let receiver = handle.add_connection(connection_id).await;
        handle.request_low_state(connection_id).await;
        handle.request_rgbd_sensors(connection_id).await;

        log::info!("Starting communication with {connection_id}");
        handle_socket(socket, receiver).await;
        log::info!("Ending communication with {connection_id}");
        handle.remove_connection(connection_id).await;
    })
}

fn serialize<T: Serialize>(data: &T) -> Option<Message> {
    match serde_json::to_string(data) {
        Ok(string) => Some(Message::Text(string.into())),
        Err(error) => {
            log::error!("Failed to serialize message: {error}");
            None
        }
    }
}

async fn handle_socket(
    mut socket: WebSocket,
    mut controller: Receiver<ControllerData>,
) -> Option<()> {
    while let Some(message) = controller.recv().await {
        match message {
            ControllerData::LowState(low_state) => {
                // TODO(oleflb): this is really bad for performance, this should be fixed by not blocking
                let data = low_state.await.unwrap();
                let data = serialize(&data)?;
                socket.send(data).await.ok()?;
            }
            ControllerData::RGBDSensors(rgbdsensors) => {
                let data = rgbdsensors.await.unwrap();
                let data = serialize(&data)?;
                socket.send(data).await.ok()?;
            }
            ControllerData::GetLowCommand(sender) => {
                let message = match socket.recv().await? {
                    Ok(message) => message,
                    Err(error) => {
                        log::error!("WebSocket error: {error}");
                        return None;
                    }
                };
                match message {
                    Message::Text(text) => {
                        let low_command: LowCommand = match serde_json::from_str(text.as_str()) {
                            Ok(command) => command,
                            Err(error) => {
                                log::error!("Failed to deserialize LowCommand: {error}");
                                continue;
                            }
                        };
                        if sender.send(low_command).is_err() {
                            log::error!("Failed to send LowCommand to controller");
                        }
                    }
                    _ => {
                        log::error!("Expected text message for LowCommand");
                    }
                }
            }
        }
    }
    Some(())
}
