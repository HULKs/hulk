use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use simulation_message::{ClientMessageKind, ConnectionInfo, ServerMessageKind, SimulationMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::{accept_async, tungstenite::Message, WebSocketStream};

use crate::controller::{ConnectionHandle, ControllerHandle, SimulationData};

pub async fn accept_websocket(stream: TcpStream, handle: ControllerHandle) -> Result<()> {
    let websocket_stream = accept_async(stream).await.wrap_err("failed to accept")?;
    start_connection(websocket_stream, handle)
        .await
        .wrap_err("websocket connection lost")?;
    Ok(())
}

async fn receive_connection_info(
    socket: &mut WebSocketStream<TcpStream>,
) -> Result<ConnectionInfo> {
    loop {
        let message = socket.next().await.wrap_err("stream closed")??;
        match message {
            Message::Text(utf8) => {
                return serde_json::from_str(utf8.as_str()).wrap_err("failed to deserialize");
            }
            Message::Binary(_) => bail!("got unexpected binary data"),
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            Message::Close(_) => bail!("close requested"),
        }
    }
}

async fn start_connection(
    mut socket: WebSocketStream<TcpStream>,
    handle: ControllerHandle,
) -> Result<()> {
    log::info!("waiting for ConnectionInfo");
    let connection_info = receive_connection_info(&mut socket)
        .await
        .wrap_err("failed to receive ConnectionInfo")?;
    let mut connection = handle
        .connect(connection_info)
        .await
        .wrap_err("failed to register at controller")?;
    log::info!("Starting connection with {}", connection.id());

    if let Err(error) = handle_socket(socket, &mut connection).await {
        log::error!("{error}");
    }
    log::info!("Ending communication with {}", connection.id());
    connection.disconnect().await;
    Ok(())
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
        Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => {}
    }

    Ok(())
}

async fn handle_send_message(
    websocket: &mut WebSocketStream<TcpStream>,
    message: SimulationData,
) -> Result<()> {
    let message = match message {
        SimulationData::SceneDescription(bytes) => Message::Binary(bytes),
        SimulationData::SceneState(text) => Message::Text(text.into()),
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

async fn handle_socket(
    mut socket: WebSocketStream<TcpStream>,
    connection: &mut ConnectionHandle,
) -> Result<()> {
    loop {
        tokio::select! {
            received = socket.next() => {
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
