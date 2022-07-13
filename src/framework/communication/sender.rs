use awaitgroup::Worker;
use futures_util::{stream::SplitSink, SinkExt};
use log::error;
use serde::Serialize;
use serde_json::{to_string, Value};
use serialize_hierarchy::HierarchyType;
use tokio::{net::TcpStream, sync::mpsc::Receiver};
use tokio_tungstenite::{
    tungstenite::{self, protocol::CloseFrame},
    WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use super::{database_subscription_manager::OutputHierarchy, Cycler, Output};

#[derive(Debug, Serialize)]
pub struct SubscribedOutput {
    pub output: Output,
    pub data: Value,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Serialize)]
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
    SetInjectedOutputResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    UnsetInjectedOutputResult {
        id: usize,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Message {
    Json { payload: Payload },
    Binary { payload: Vec<u8> },
    Close { frame: Option<CloseFrame<'static>> },
}

pub async fn sender(
    mut writer: SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>,
    _wait_group_worker: Worker, // will be dropped when this function exits
    keep_only_self_running: CancellationToken,
    mut message_receiver: Receiver<Message>,
) {
    // this task needs to be executed as long as possible to drain the channel
    while let Some(message) = message_receiver.recv().await {
        let message = match message {
            Message::Json { payload } => {
                let message_string = match to_string(&payload) {
                    Ok(message_string) => message_string,
                    Err(error) => {
                        error!("Failed to serialize message: {:?}", error);
                        continue;
                    }
                };
                tungstenite::Message::Text(message_string)
            }
            Message::Binary { payload } => tungstenite::Message::Binary(payload),
            Message::Close { frame } => tungstenite::Message::Close(frame),
        };
        match writer.send(message).await {
            Ok(_) => {}
            Err(error) => {
                error!("Failed to write to websocket: {:?}", error);
                keep_only_self_running.cancel();
            }
        }
    }
}
