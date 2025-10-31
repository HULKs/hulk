use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use serde::Serialize;
use simulation_message::{ClientMessageKind, ServerMessageKind, SimulationMessage};

use crate::controller::{ConnectionHandle, ControllerHandle, SimulationData};

pub fn setup(handle: ControllerHandle) -> Router {
    Router::new()
        .route("/subscribe", get(ws_connection))
        .layer(Extension(handle))
}

async fn ws_connection(
    Extension(handle): Extension<ControllerHandle>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(async move |socket| {
        let mut connection = match handle.connect().await {
            Ok(connection) => connection,
            Err(error) => {
                log::error!("failed to create connection: {error}");
                return;
            }
        };
        log::info!("Starting connection with {}", connection.id());

        // 1. Get ConnectionInfo
        // TODO(oleflb)

        if let Err(error) = handle_socket(socket, &mut connection).await {
            log::error!("{error}");
        }
        log::info!("Ending communication with {}", connection.id());
    })
}

fn serialize<T: Serialize>(data: &SimulationMessage<T>) -> Result<Message> {
    serde_json::to_string(data)
        .map(|string| Message::Text(string.into()))
        .wrap_err("failed to serialize data")
}

async fn handle_received_message(connection: &ConnectionHandle, message: Message) -> Result<()> {
    match message {
        Message::Text(text) => {
            let data = serde_json::from_str(&text).wrap_err("failed to deserialize")?;
            match data {
                ClientMessageKind::LowCommand(low_command) => {
                    connection.send_low_command(low_command).await?;
                }
            }
        }
        Message::Binary(_) => log::info!("received unexpected binary data"),
        Message::Ping(_) | Message::Pong(_) | Message::Close(_) => {}
    }

    Ok(())
}

async fn handle_send_message(websocket: &mut WebSocket, message: SimulationData) -> Result<()> {
    let message = match message {
        SimulationData::SceneDescription(bytes) | SimulationData::SceneState(bytes) => {
            Message::Binary(bytes)
        }
        SimulationData::LowState { time, data } => {
            let message = SimulationMessage {
                time,
                payload: ServerMessageKind::LowState(data),
            };
            serialize(&message)?
        }
        SimulationData::Image { time, data } => {
            let message = SimulationMessage {
                time,
                payload: ServerMessageKind::RGBDSensors(data),
            };
            serialize(&message)?
        }
    };
    websocket
        .send(message)
        .await
        .wrap_err("failed to send into websocket")
}

async fn handle_socket(mut socket: WebSocket, connection: &mut ConnectionHandle) -> Result<()> {
    loop {
        tokio::select! {
            received = socket.recv() => {
                let message = received.wrap_err("websocket closed")??;
                handle_received_message(connection, message).await?;
            },
            to_send = connection.receive_data() => {
                let message = to_send?;
                handle_send_message(&mut socket, message).await?;
            }
        }
    }
}
