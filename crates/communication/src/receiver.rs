use futures_util::{stream::SplitStream, StreamExt};
use log::{debug, error};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{connector, parameter_subscription_manager, types::SubscribedOutput};

use super::{output_subscription_manager, responder, Cycler};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum Payload {
    GetOutputHierarchyResult {
        id: usize,
        ok: bool,
        output_hierarchy: Value,
    },
    SubscribeOutputResult {
        id: usize,
        ok: bool,
        reason: Option<String>,
    },
    UnsubscribeOutputResult {
        id: usize,
        ok: bool,
        reason: Option<String>,
    },
    OutputsUpdated {
        cycler: Cycler,
        outputs: Vec<SubscribedOutput>,
        image_id: Option<u32>,
    },
    GetParameterHierarchyResult {
        id: usize,
        ok: bool,
        parameter_hierarchy: Value,
    },
    SubscribeParameterResult {
        id: usize,
        ok: bool,
        reason: Option<String>,
    },
    UnsubscribeParameterResult {
        id: usize,
        ok: bool,
        reason: Option<String>,
    },
    UpdateParameterResult {
        id: usize,
        ok: bool,
        reason: Option<String>,
    },
    ParameterUpdated {
        path: String,
        data: Value,
    },
}

pub async fn receiver(
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    responder: Sender<responder::Message>,
    output_subscription_manager: Sender<output_subscription_manager::Message>,
    parameter_subscription_manager: Sender<parameter_subscription_manager::Message>,
    connector: Sender<connector::Message>,
) {
    while let Some(message) = reader.next().await {
        debug!("Receiver got message: {message:?}");
        match message {
            Ok(message) => match message {
                tokio_tungstenite::tungstenite::Message::Text(content) => {
                    let payload = match serde_json::from_str::<Payload>(&content) {
                        Ok(payload) => payload,
                        Err(error) => {
                            error!("Failed to deserialize message content: {error:?}\nMessage was {content:#?}");
                            continue;
                        }
                    };
                    match payload {
                        Payload::GetOutputHierarchyResult {
                            id,
                            ok,
                            output_hierarchy,
                        } => {
                            let response = result_from_response(ok, None, output_hierarchy);
                            responder
                                .send(responder::Message::Respond { id, response })
                                .await
                                .unwrap();
                        }
                        Payload::OutputsUpdated {
                            cycler,
                            outputs,
                            image_id: _,
                        } => {
                            output_subscription_manager
                                .send(output_subscription_manager::Message::Update {
                                    cycler,
                                    outputs,
                                })
                                .await
                                .unwrap();
                        }
                        Payload::GetParameterHierarchyResult {
                            id,
                            ok,
                            parameter_hierarchy: hierarchy,
                        } => {
                            let response = result_from_response(ok, None, hierarchy);
                            responder
                                .send(responder::Message::Respond { id, response })
                                .await
                                .unwrap();
                        }
                        Payload::ParameterUpdated { path, data } => {
                            parameter_subscription_manager
                                .send(parameter_subscription_manager::Message::Update {
                                    path,
                                    data,
                                })
                                .await
                                .unwrap();
                        }
                        Payload::SubscribeOutputResult { id, ok, reason }
                        | Payload::UnsubscribeOutputResult { id, ok, reason }
                        | Payload::SubscribeParameterResult { id, ok, reason }
                        | Payload::UnsubscribeParameterResult { id, ok, reason }
                        | Payload::UpdateParameterResult { id, ok, reason } => {
                            let response =
                                result_from_response(ok, reason, Value::Object(Map::new()));
                            responder
                                .send(responder::Message::Respond { id, response })
                                .await
                                .unwrap();
                        }
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                    break;
                }
                _ => {
                    error!("Got unsupported message type from socket");
                    break;
                }
            },
            Err(error) => {
                error!("Error while receiving message from socket: {error:?}");
                output_subscription_manager
                    .send(output_subscription_manager::Message::Disconnect)
                    .await
                    .unwrap();
                connector
                    .send(connector::Message::ConnectionFailed {
                        info: "Peer disconnected".to_string(),
                    })
                    .await
                    .unwrap();
            }
        }
    }
}

fn result_from_response(ok: bool, reason: Option<String>, value: Value) -> Result<Value, String> {
    match ok {
        true => Ok(value),
        false => match reason {
            Some(reason) => Err(reason),
            None => Err("No reason specified".to_string()),
        },
    }
}
