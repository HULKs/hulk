use bincode::serialize;
use futures_util::{stream::SplitSink, SinkExt};
use serde_json::to_string;
use tokio::{
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};
use tokio_tungstenite::{
    tungstenite::{protocol::CloseFrame, Message},
    WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use crate::messages::Response;

use super::connection::ReceiverOrSenderError;

pub async fn sender(
    mut writer: SplitSink<WebSocketStream<TcpStream>, Message>,
    error_sender: Sender<ReceiverOrSenderError>,
    keep_only_self_running: CancellationToken,
    mut response_receiver: Receiver<Response>,
) {
    while let Some(response) = response_receiver.recv().await {
        let message = match response {
            Response::Textual(textual) => {
                let message_string = match to_string(&textual) {
                    Ok(message_string) => message_string,
                    Err(error) => {
                        error_sender
                            .send(ReceiverOrSenderError::JsonNotSerialized(error))
                            .await
                            .expect("receiver should always wait for all senders");
                        keep_only_self_running.cancel();
                        continue;
                    }
                };

                Message::Text(message_string)
            }
            Response::Binary(binary) => {
                let message_bytes = match serialize(&binary) {
                    Ok(message_bytes) => message_bytes,
                    Err(error) => {
                        error_sender
                            .send(ReceiverOrSenderError::BincodeNotSerialized(error))
                            .await
                            .expect("receiver should always wait for all senders");
                        keep_only_self_running.cancel();
                        continue;
                    }
                };

                Message::Binary(message_bytes)
            }
            Response::Close { code, reason } => Message::Close(Some(CloseFrame {
                code,
                reason: reason.into(),
            })),
        };

        match writer.send(message).await {
            Ok(_) => {}
            Err(error) => {
                error_sender
                    .send(ReceiverOrSenderError::WebSocketMessageNotWritten(error))
                    .await
                    .expect("receiver should always wait for all senders");
                keep_only_self_running.cancel();
            }
        }
    }
}
