use futures_util::{stream::SplitStream, StreamExt};
use serde_json::from_str;
use tokio::{net::TcpStream, select, sync::mpsc::Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tokio_util::sync::CancellationToken;

use crate::server::databases::{Client, ClientRequest};

use super::{
    connection::ReceiverOrSenderError,
    databases,
    messages::{Request, Response},
};

pub async fn receiver(
    mut reader: SplitStream<WebSocketStream<TcpStream>>,
    error_sender: Sender<ReceiverOrSenderError>,
    keep_running: CancellationToken,
    keep_only_self_running: CancellationToken,
    client_id: usize,
    response_sender: Sender<Response>,
    databases_sender: Sender<databases::Request>,
) {
    select! {
        _ = async {
            while let Some(message) = reader.next().await {
                handle_message(
                    message,
                    &error_sender,
                    &keep_only_self_running,
                    client_id,
                    &response_sender,
                    &databases_sender,
                ).await;
            }
        } => {},
        _ = keep_running.cancelled() => {},
        _ = keep_only_self_running.cancelled() => {},
    }

    // TODO: Unsubscribe everything
}

async fn handle_message(
    message: Result<Message, tokio_tungstenite::tungstenite::Error>,
    error_sender: &Sender<ReceiverOrSenderError>,
    keep_only_self_running: &CancellationToken,
    client_id: usize,
    response_sender: &Sender<Response>,
    databases_sender: &Sender<databases::Request>,
) {
    let message = match message {
        Ok(message) => message,
        Err(error) => {
            error_sender
                .send(ReceiverOrSenderError::WebSocketMessageNotRead(error))
                .await
                .expect("receiver should always wait for all senders");
            // send_close_from_error("Failed to read from websocket", error, message_sender).await;
            keep_only_self_running.cancel();
            return;
        }
    };

    match message {
        Message::Text(message) => {
            let request: Request = match from_str(&message) {
                Ok(request) => request,
                Err(error) => {
                    error_sender
                        .send(ReceiverOrSenderError::JsonNotDeserialized(error))
                        .await
                        .expect("receiver should always wait for all senders");
                    // send_close_from_error("Failed to read from websocket", error, message_sender).await;
                    keep_only_self_running.cancel();
                    return;
                }
            };

            match request {
                Request::Databases(request) => {
                    println!("receiver: request: {request:?}");
                    databases_sender
                        .send(databases::Request::ClientRequest(ClientRequest {
                            request,
                            client: Client {
                                id: client_id,
                                response_sender: response_sender.clone(),
                            },
                        }))
                        .await
                        .expect("receiver should always wait for all senders");
                }
                Request::Injections(_) => todo!(),
                Request::Parameters(_) => todo!(),
            }
        }
        Message::Binary(_) => {
            error_sender
                .send(ReceiverOrSenderError::GotUnexpectedBinaryMessage)
                .await
                .expect("receiver should always wait for all senders");
            // send_close_from_error("Failed to read from websocket", error, message_sender).await;
            keep_only_self_running.cancel();
            return;
        }
        _ => {}
    }
}
