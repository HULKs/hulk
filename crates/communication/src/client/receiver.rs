use byteorder::{ByteOrder, LittleEndian};
use futures_util::{stream::SplitStream, StreamExt};
use log::{debug, error, info};
use serde::Deserialize;
use serde_json::{Map, Value};
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::{
    client::{connector, parameter_subscription_manager, types::SubscribedOutput},
    messages::{TextualOutputResponse, TextualResponse},
};

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
                    let message = match serde_json::from_str::<TextualResponse>(&content) {
                        Ok(payload) => payload,
                        Err(error) => {
                            error!("Failed to deserialize message content: {error:?}\nMessage was {content:#?}");
                            continue;
                        }
                    };
                    println!("{message:?}");
                    match message {
                        TextualResponse::Outputs(outputs_message) => match outputs_message {
                            TextualOutputResponse::GetFields { id, fields } => {
                                if let Err(error) = responder
                                    .send(responder::Message::Respond {
                                        id,
                                        response: responder::Response::Fields(fields),
                                    })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
                            TextualOutputResponse::GetNext { id, result } => todo!(),
                            TextualOutputResponse::Subscribe { id, result } => {
                                let response = responder::Response::Subscribe(result);
                                if let Err(error) = responder
                                    .send(responder::Message::Respond { id, response })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
                            TextualOutputResponse::Unsubscribe { id, result } => {
                                let response = responder::Response::Unsubscribe(result);
                                if let Err(error) = responder
                                    .send(responder::Message::Respond { id, response })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
,
                            TextualOutputResponse::SubscribedData { items } => {
                                if let Err(error) = output_subscription_manager
                                    .send(output_subscription_manager::Message::Update { items })
                                    .await
                                {
                                    error!("{error}");
                                }
                            }
                        },
                        message => todo!("unimplemented message {message:?}"),
                    }
                    //         Payload::GetOutputHierarchyResult {
                    //             id,
                    //             ok,
                    //             output_hierarchy,
                    //         } => {
                    //             let response = result_from_response(ok, None, output_hierarchy);
                    //             if let Err(error) = responder
                    //                 .send(responder::Message::Respond { id, response })
                    //                 .await
                    //             {
                    //                 error!("{error}");
                    //             }
                    //         }
                    //         Payload::OutputsUpdated {
                    //             cycler,
                    //             outputs,
                    //             image_id,
                    //         } => {
                    //             if let Err(error) = output_subscription_manager
                    //                 .send(output_subscription_manager::Message::Update {
                    //                     cycler,
                    //                     outputs,
                    //                     image_id,
                    //                 })
                    //                 .await
                    //             {
                    //                 error!("{error}");
                    //             }
                    //         }
                    //         Payload::GetParameterHierarchyResult {
                    //             id,
                    //             ok,
                    //             parameter_hierarchy: hierarchy,
                    //         } => {
                    //             let response = result_from_response(ok, None, hierarchy);
                    //             if let Err(error) = responder
                    //                 .send(responder::Message::Respond { id, response })
                    //                 .await
                    //             {
                    //                 error!("{error}");
                    //             }
                    //         }
                    //         Payload::ParameterUpdated { path, data } => {
                    //             if let Err(error) = parameter_subscription_manager
                    //                 .send(parameter_subscription_manager::Message::Update {
                    //                     path,
                    //                     data,
                    //                 })
                    //                 .await
                    //             {
                    //                 error!("{error}");
                    //             }
                    //         }
                    //         Payload::SubscribeOutputResult { id, ok, reason }
                    //         | Payload::UnsubscribeOutputResult { id, ok, reason }
                    //         | Payload::SubscribeParameterResult { id, ok, reason }
                    //         | Payload::UnsubscribeParameterResult { id, ok, reason }
                    //         | Payload::UpdateParameterResult { id, ok, reason } => {
                    //             let response =
                    //                 result_from_response(ok, reason, Value::Object(Map::new()));
                    //             if let Err(error) = responder
                    //                 .send(responder::Message::Respond { id, response })
                    //                 .await
                    //             {
                    //                 error!("{error}");
                    //             }
                    //         }
                    //     }
                }
                tokio_tungstenite::tungstenite::Message::Close(close_frame) => {
                    info!("closed: {close_frame:?}");
                    break;
                }
                tokio_tungstenite::tungstenite::Message::Binary(data) => {
                    let length = LittleEndian::read_u32(&data[0..4]);
                    let image_id = LittleEndian::read_u32(&data[4..8]) as usize;
                    let data = data[8..].to_vec();
                    assert_eq!(length as usize, data.len());

                    output_subscription_manager
                        .send(output_subscription_manager::Message::UpdateImage { image_id, data })
                        .await
                        .unwrap();
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
