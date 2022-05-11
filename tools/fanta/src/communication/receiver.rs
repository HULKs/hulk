use std::collections::BTreeMap;

use anyhow::anyhow;
use futures_util::{stream::SplitStream, StreamExt};
use log::{debug, error};
use serde::Deserialize;
use serde_json::Value;
use tokio::{net::TcpStream, sync::mpsc::Sender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::{manager, responder, Cycler, Output};

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum HierarchyType {
    Primary {
        name: String,
    },
    Struct {
        fields: BTreeMap<String, HierarchyType>,
    },
    GenericStruct,
    Option {
        nested: Box<HierarchyType>,
    },
    Vec {
        nested: Box<HierarchyType>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct CyclerOutputsHierarchy {
    pub main: HierarchyType,
    pub additional: HierarchyType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OutputHierarchy {
    pub control: CyclerOutputsHierarchy,
    pub vision_top: CyclerOutputsHierarchy,
    pub vision_bottom: CyclerOutputsHierarchy,
}

#[derive(Debug, Deserialize)]
pub struct SubscribedOutput {
    pub output: Output,
    pub data: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Payload {
    GetOutputHierarchyResult {
        id: usize,
        ok: bool,
        output_hierarchy: OutputHierarchy,
    },
    SubscribeOutputResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    UnsubscribeOutputResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    OutputsUpdated {
        cycler: Cycler,
        outputs: Vec<SubscribedOutput>,
        #[serde(skip_serializing_if = "Option::is_none")]
        image_id: Option<u32>,
    },
    GetParameterHierarchyResult {
        id: usize,
        ok: bool,
        parameter_hierarchy: HierarchyType,
    },
    SubscribeParameterResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    UnsubscribeParameterResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    UpdateParameterResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    ParameterUpdated {
        path: String,
        data: Value,
    },
}

pub async fn receiver(
    mut reader: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    responder_sender: Sender<responder::Message>,
    manager_sender: Sender<manager::Message>,
) {
    while let Some(message) = reader.next().await {
        debug!("Receiver got message: {message:?}");
        match message {
            Ok(message) => match message {
                tokio_tungstenite::tungstenite::Message::Text(content) => {
                    let payload = match serde_json::from_str::<Payload>(&content) {
                        Ok(payload) => payload,
                        Err(error) => {
                            error!("Failed to deserialize message content: {error:?}");
                            continue;
                        }
                    };
                    match payload {
                        Payload::GetOutputHierarchyResult {
                            id: _,
                            ok: _,
                            output_hierarchy: _,
                        } => todo!(),
                        Payload::SubscribeOutputResult { id, ok, reason } => {
                            let response = if ok {
                                Ok(())
                            } else {
                                Err(match reason {
                                    Some(reason) => anyhow!("reason: {reason}"),
                                    None => anyhow!("No reason specified"),
                                })
                            };
                            if let Err(error) = responder_sender
                                .send(responder::Message::Respond { id, response })
                                .await
                            {
                                error!("Failed to send response to responder channel: {error:?}");
                            };
                        }
                        Payload::UnsubscribeOutputResult {
                            id: _,
                            ok: _,
                            reason: _,
                        } => todo!(),
                        Payload::OutputsUpdated {
                            cycler,
                            outputs,
                            image_id: _,
                        } => {
                            if let Err(error) = manager_sender
                                .send(manager::Message::OutputsUpdated { cycler, outputs })
                                .await
                            {
                                error!("Failed to send updated outputs to manager: {error:?}");
                            }
                        }
                        Payload::GetParameterHierarchyResult {
                            id: _,
                            ok: _,
                            parameter_hierarchy: _,
                        } => todo!(),
                        Payload::SubscribeParameterResult {
                            id: _,
                            ok: _,
                            reason: _,
                        } => todo!(),
                        Payload::UnsubscribeParameterResult {
                            id: _,
                            ok: _,
                            reason: _,
                        } => todo!(),
                        Payload::UpdateParameterResult {
                            id: _,
                            ok: _,
                            reason: _,
                        } => todo!(),
                        Payload::ParameterUpdated { path: _, data: _ } => todo!(),
                    }
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => todo!(),
                _ => error!("Got unsupported message type from socket"),
            },
            Err(error) => error!("Error while receiving message from socket: {error:?}"),
        }
    }
}
