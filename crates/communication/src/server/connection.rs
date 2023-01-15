use std::{io, net::SocketAddr};

use futures_util::StreamExt;
use log::error;
use tokio::{
    net::TcpStream,
    select, spawn,
    sync::mpsc::{channel, Sender, UnboundedSender},
};
use tokio_tungstenite::accept_async;
use tokio_util::sync::CancellationToken;

use super::{outputs, receiver::receiver, sender::sender};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("failed to get peer address of TCP stream")]
    PeerAddressNotAvailable(io::Error),
    #[error("encountered error in connection {peer_address}")]
    ReceiverOrSenderError {
        source: ReceiverOrSenderError,
        peer_address: SocketAddr,
    },
    #[error("failed to accept WebSocket connection {peer_address} (handshake)")]
    WebSocketConnectionNotAccepted {
        source: tokio_tungstenite::tungstenite::Error,
        peer_address: SocketAddr,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ReceiverOrSenderError {
    #[error("failed to serialize Bincode")]
    BincodeNotSerialized(bincode::Error),
    #[error("got unexpected binary message")]
    GotUnexpectedBinaryMessage,
    #[error("failed to deserialize JSON")]
    JsonNotDeserialized(serde_json::Error),
    #[error("failed to serialize JSON")]
    JsonNotSerialized(serde_json::Error),
    #[error("failed to read WebSocket message")]
    WebSocketMessageNotRead(tokio_tungstenite::tungstenite::Error),
    #[error("failed to write WebSocket message")]
    WebSocketMessageNotWritten(tokio_tungstenite::tungstenite::Error),
}

pub(crate) fn connection(
    stream: TcpStream,
    keep_running: CancellationToken,
    connection_error_sender: UnboundedSender<ConnectionError>,
    outputs_sender: Sender<outputs::Request>,
    client_id: usize,
) {
    spawn(async move {
        let peer_address = match stream.peer_addr() {
            Ok(peer_address) => peer_address,
            Err(error) => {
                connection_error_sender
                    .send(ConnectionError::PeerAddressNotAvailable(error))
                    .expect("receiver should always wait for all senders");
                return;
            }
        };

        let websocket_stream = select! {
            result = accept_async(stream) => match result {
                Ok(websocket_stream) => websocket_stream,
                Err(source) => {
                    connection_error_sender
                        .send(ConnectionError::WebSocketConnectionNotAccepted{ source, peer_address })
                        .expect("receiver should always wait for all senders");
                    return;
                }
            },
            _ = keep_running.cancelled() => return,
        };

        let (writer, reader) = websocket_stream.split();

        let (receiver_or_sender_error_sender, mut receiver_or_sender_error_receiver) = channel(1);
        let keep_only_self_running = CancellationToken::new();
        let (response_sender, response_receiver) = channel(1);

        spawn(receiver(
            reader,
            receiver_or_sender_error_sender.clone(),
            keep_running,
            keep_only_self_running.clone(),
            client_id,
            response_sender,
            outputs_sender,
        ));

        spawn(sender(
            writer,
            receiver_or_sender_error_sender,
            keep_only_self_running,
            response_receiver,
        ));

        while let Some(error) = receiver_or_sender_error_receiver.recv().await {
            error!("Error from connection {peer_address}: {error}");
            connection_error_sender
                .send(ConnectionError::ReceiverOrSenderError {
                    source: error,
                    peer_address,
                })
                .expect("receiver should always wait for all senders");
        }
    });
}
