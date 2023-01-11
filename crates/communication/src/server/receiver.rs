use futures_util::{stream::SplitStream, StreamExt};
use log::info;
use serde_json::from_str;
use tokio::{net::TcpStream, select, sync::mpsc::Sender};
use tokio_tungstenite::{
    tungstenite::{protocol::frame::coding::CloseCode, Message},
    WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use crate::{
    messages::{OutputsRequest, ParametersRequest, Request, Response},
    server::client_request::ClientRequest,
};

use super::{client::Client, connection::ReceiverOrSenderError, outputs};

#[allow(clippy::too_many_arguments)]
pub async fn receiver(
    mut reader: SplitStream<WebSocketStream<TcpStream>>,
    error_sender: Sender<ReceiverOrSenderError>,
    keep_running: CancellationToken,
    keep_only_self_running: CancellationToken,
    client_id: usize,
    response_sender: Sender<Response>,
    outputs_sender: Sender<outputs::Request>,
    parameters_sender: Sender<ClientRequest<ParametersRequest>>,
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
                    &outputs_sender,
                    &parameters_sender,
                ).await;
            }
        } => {},
        _ = keep_running.cancelled() => {},
        _ = keep_only_self_running.cancelled() => {},
    }

    outputs_sender
        .send(outputs::Request::ClientRequest(ClientRequest {
            request: OutputsRequest::UnsubscribeEverything,
            client: Client {
                id: client_id,
                response_sender: response_sender.clone(),
            },
        }))
        .await
        .expect("receiver should always wait for all senders");
    parameters_sender
        .send(ClientRequest {
            request: ParametersRequest::UnsubscribeEverything,
            client: Client {
                id: client_id,
                response_sender: response_sender.clone(),
            },
        })
        .await
        .expect("receiver should always wait for all senders");
}

async fn handle_message(
    message: Result<Message, tokio_tungstenite::tungstenite::Error>,
    error_sender: &Sender<ReceiverOrSenderError>,
    keep_only_self_running: &CancellationToken,
    client_id: usize,
    response_sender: &Sender<Response>,
    outputs_sender: &Sender<outputs::Request>,
    parameters_sender: &Sender<ClientRequest<ParametersRequest>>,
) {
    let message = match message {
        Ok(message) => message,
        Err(error) => {
            send_error(
                ReceiverOrSenderError::WebSocketMessageNotRead(error),
                error_sender,
                response_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };

    match message {
        Message::Text(message) => {
            let request: Request = match from_str(&message) {
                Ok(request) => request,
                Err(error) => {
                    info!("message was: {message}");
                    send_error(
                        ReceiverOrSenderError::JsonNotDeserialized(error),
                        error_sender,
                        response_sender,
                    )
                    .await;
                    keep_only_self_running.cancel();
                    return;
                }
            };

            let client = Client {
                id: client_id,
                response_sender: response_sender.clone(),
            };
            match request {
                Request::Outputs(request) => {
                    outputs_sender
                        .send(outputs::Request::ClientRequest(ClientRequest {
                            request,
                            client,
                        }))
                        .await
                        .expect("receiver should always wait for all senders");
                }
                Request::Injections(_) => todo!(),
                Request::Parameters(request) => {
                    parameters_sender
                        .send(ClientRequest { request, client })
                        .await
                        .expect("receiver should always wait for all senders");
                }
            }
        }
        Message::Binary(_) => {
            send_error(
                ReceiverOrSenderError::GotUnexpectedBinaryMessage,
                error_sender,
                response_sender,
            )
            .await;
            keep_only_self_running.cancel();
        }
        _ => {}
    }
}

async fn send_error(
    error: ReceiverOrSenderError,
    error_sender: &Sender<ReceiverOrSenderError>,
    response_sender: &Sender<Response>,
) {
    let reason = error.to_string();
    error_sender
        .send(error)
        .await
        .expect("receiver should always wait for all senders");
    response_sender
        .send(Response::Close {
            code: CloseCode::Error,
            reason,
        })
        .await
        .expect("receiver should always wait for all senders");
}
