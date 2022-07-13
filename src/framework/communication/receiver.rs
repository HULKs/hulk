use std::{borrow::Cow, fmt::Debug, net::SocketAddr};

use awaitgroup::Worker;
use futures_util::{stream::SplitStream, StreamExt};
use log::{error, warn};
use serde::Deserialize;
use serde_json::{from_str, Value};
use tokio::{
    net::TcpStream,
    select,
    sync::{
        mpsc::Sender,
        oneshot::{self, channel},
    },
};
use tokio_tungstenite::{
    tungstenite::{
        self,
        protocol::{frame::coding::CloseCode, CloseFrame},
        Error,
    },
    WebSocketStream,
};
use tokio_util::sync::CancellationToken;

use super::{
    database_subscription_manager, injection_writer, parameter_modificator,
    sender::{Message, Payload},
    Cycler, CyclerOutput,
};

#[allow(clippy::too_many_arguments)]
pub async fn receiver(
    peer_address: SocketAddr,
    mut reader: SplitStream<WebSocketStream<TcpStream>>,
    database_subscription_manager_sender: Sender<database_subscription_manager::Request>,
    parameter_modificator_sender: Sender<parameter_modificator::Request>,
    injection_writer_sender: Sender<injection_writer::Request>,
    _wait_group_worker: Worker, // will be dropped when this function exits
    keep_running: CancellationToken,
    keep_only_self_running: CancellationToken,
    message_sender: Sender<Message>,
) {
    select! {
        _ = async {
            while let Some(message) = reader.next().await {
                handle_message(
                    message,
                    &peer_address,
                    &database_subscription_manager_sender,
                    &parameter_modificator_sender,
                    &injection_writer_sender,
                    &keep_only_self_running,
                    &message_sender,
                ).await;
            }
        } => {},
        _ = keep_running.cancelled() => {},
        _ = keep_only_self_running.cancelled() => {},
    };

    let request = database_subscription_manager::Request::UnsubscribeEverything {
        client: peer_address,
    };
    if let Err(error) = database_subscription_manager_sender.send(request).await {
        send_close_from_error(
            "Failed to send request, closing now",
            error,
            &message_sender,
        )
        .await;
    }

    let request = parameter_modificator::Request::UnsubscribeEverything {
        client: peer_address,
    };
    if let Err(error) = parameter_modificator_sender.send(request).await {
        send_close_from_error(
            "Failed to send request, closing now",
            error,
            &message_sender,
        )
        .await;
    }
}

async fn send_close_from_error<E>(message: &'static str, error: E, message_sender: &Sender<Message>)
where
    E: Debug,
{
    error!("{}: {:?}", message, error);
    if let Err(error) = message_sender
        .send(Message::Close {
            frame: Some(CloseFrame {
                code: CloseCode::Error,
                reason: Cow::from(message),
            }),
        })
        .await
    {
        error!(
            "Failed to send close message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_message(
    message: Result<tungstenite::Message, Error>,
    peer_address: &SocketAddr,
    database_subscription_manager_sender: &Sender<database_subscription_manager::Request>,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    injection_writer_sender: &Sender<injection_writer::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let message = match message {
        Ok(message) => message,
        Err(error) => {
            send_close_from_error("Failed to read from websocket", error, message_sender).await;
            keep_only_self_running.cancel();
            return;
        }
    };

    match message {
        tungstenite::Message::Text(message) => {
            handle_text_message(
                message,
                peer_address,
                database_subscription_manager_sender,
                parameter_modificator_sender,
                injection_writer_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        tungstenite::Message::Binary(_) => {
            handle_binary_message(keep_only_self_running, message_sender).await;
        }
        _ => {}
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Request {
    GetOutputHierarchy {
        id: usize,
    },
    SubscribeOutput {
        id: usize,
        output: CyclerOutput,
    },
    UnsubscribeOutput {
        id: usize,
        output: CyclerOutput,
    },
    GetParameterHierarchy {
        id: usize,
    },
    SubscribeParameter {
        id: usize,
        path: String,
    },
    UnsubscribeParameter {
        id: usize,
        path: String,
    },
    UpdateParameter {
        id: usize,
        path: String,
        data: Value,
    },
    SetInjectedOutput {
        id: usize,
        cycler: Cycler,
        path: String,
        data: Value,
    },
    UnsetInjectedOutput {
        id: usize,
        cycler: Cycler,
        path: String,
    },
}

pub fn respond_or_log_error<T>(response_sender: oneshot::Sender<T>, item: T)
where
    T: Debug,
{
    if let Err(error) = response_sender.send(item) {
        error!(
            "Failed to send message into channel for response: {:?}",
            error
        );
    }
}

async fn handle_text_message(
    message: String,
    peer_address: &SocketAddr,
    database_subscription_manager_sender: &Sender<database_subscription_manager::Request>,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    injection_writer_sender: &Sender<injection_writer::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let request: Request = match from_str(&message) {
        Ok(request) => request,
        Err(error) => {
            send_close_from_error("Failed to parse from websocket", error, message_sender).await;
            keep_only_self_running.cancel();
            return;
        }
    };

    match request {
        Request::GetOutputHierarchy { id } => {
            handle_get_output_hierarchy_request(
                id,
                database_subscription_manager_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::SubscribeOutput { id, output } => {
            handle_subscribe_output_request(
                id,
                output,
                peer_address,
                database_subscription_manager_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::UnsubscribeOutput { id, output } => {
            handle_unsubscribe_output_request(
                id,
                output,
                peer_address,
                database_subscription_manager_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::GetParameterHierarchy { id } => {
            handle_get_parameter_hierarchy_request(
                id,
                parameter_modificator_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::SubscribeParameter { id, path } => {
            handle_subscribe_parameter_request(
                id,
                path,
                peer_address,
                parameter_modificator_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::UnsubscribeParameter { id, path } => {
            handle_unsubscribe_parameter_request(
                id,
                path,
                peer_address,
                parameter_modificator_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::UpdateParameter { id, path, data } => {
            handle_update_parameter_request(
                id,
                path,
                data,
                parameter_modificator_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::SetInjectedOutput {
            id,
            cycler,
            path,
            data,
        } => {
            handle_set_injected_output_request(
                id,
                cycler,
                path,
                data,
                injection_writer_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
        Request::UnsetInjectedOutput { id, cycler, path } => {
            handle_unset_injected_output_request(
                id,
                cycler,
                path,
                injection_writer_sender,
                keep_only_self_running,
                message_sender,
            )
            .await;
        }
    }
}

async fn handle_get_output_hierarchy_request(
    id: usize,
    database_subscription_manager_sender: &Sender<database_subscription_manager::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = database_subscription_manager::Request::GetOutputHierarchy { response_sender };
    if let Err(error) = database_subscription_manager_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(output_hierarchy) => Payload::GetOutputHierarchyResult {
            id,
            ok: true,
            output_hierarchy,
        },
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_subscribe_output_request(
    id: usize,
    output: CyclerOutput,
    peer_address: &SocketAddr,
    database_subscription_manager_sender: &Sender<database_subscription_manager::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = database_subscription_manager::Request::SubscribeOutput {
        client: *peer_address,
        output: output.clone(),
        response_sender,
        output_sender: message_sender.clone(),
    };
    if let Err(error) = database_subscription_manager_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::SubscribeOutputResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::SubscribeOutputResult {
            id,
            ok: false,
            reason: Some(format!("Failed to subscribe to {:?}: {:?}", output, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_unsubscribe_output_request(
    id: usize,
    output: CyclerOutput,
    peer_address: &SocketAddr,
    database_subscription_manager_sender: &Sender<database_subscription_manager::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = database_subscription_manager::Request::UnsubscribeOutput {
        client: *peer_address,
        output: output.clone(),
        response_sender,
    };
    if let Err(error) = database_subscription_manager_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::UnsubscribeOutputResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::UnsubscribeOutputResult {
            id,
            ok: false,
            reason: Some(format!(
                "Failed to unsubscribe from {:?}: {:?}",
                output, error
            )),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_get_parameter_hierarchy_request(
    id: usize,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = parameter_modificator::Request::GetParameterHierarchy { response_sender };
    if let Err(error) = parameter_modificator_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(parameter_hierarchy) => Payload::GetParameterHierarchyResult {
            id,
            ok: true,
            parameter_hierarchy,
        },
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_subscribe_parameter_request(
    id: usize,
    path: String,
    peer_address: &SocketAddr,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = parameter_modificator::Request::SubscribeParameter {
        client: *peer_address,
        path: path.clone(),
        response_sender,
        parameter_sender: message_sender.clone(),
    };
    if let Err(error) = parameter_modificator_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::SubscribeParameterResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::SubscribeParameterResult {
            id,
            ok: false,
            reason: Some(format!("Failed to subscribe to {:?}: {:?}", path, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_unsubscribe_parameter_request(
    id: usize,
    path: String,
    peer_address: &SocketAddr,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = parameter_modificator::Request::UnsubscribeParameter {
        client: *peer_address,
        path: path.clone(),
        response_sender,
    };
    if let Err(error) = parameter_modificator_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::UnsubscribeParameterResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::UnsubscribeParameterResult {
            id,
            ok: false,
            reason: Some(format!("Failed to unsubscribe to {:?}: {:?}", path, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_update_parameter_request(
    id: usize,
    path: String,
    data: Value,
    parameter_modificator_sender: &Sender<parameter_modificator::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = parameter_modificator::Request::UpdateParameter {
        path: path.clone(),
        data,
        response_sender,
    };
    if let Err(error) = parameter_modificator_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::UpdateParameterResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::UpdateParameterResult {
            id,
            ok: false,
            reason: Some(format!("Failed to update at {:?}: {:?}", path, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_set_injected_output_request(
    id: usize,
    cycler: Cycler,
    path: String,
    data: Value,
    injection_writer_sender: &Sender<injection_writer::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = injection_writer::Request::SetInjectedOutput {
        cycler,
        path: path.clone(),
        data,
        response_sender,
    };
    if let Err(error) = injection_writer_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::SetInjectedOutputResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::SetInjectedOutputResult {
            id,
            ok: false,
            reason: Some(format!("Failed to update at {:?}: {:?}", path, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_unset_injected_output_request(
    id: usize,
    cycler: Cycler,
    path: String,
    injection_writer_sender: &Sender<injection_writer::Request>,
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    let (response_sender, response_receiver) = channel();
    let request = injection_writer::Request::UnsetInjectedOutput {
        cycler,
        path: path.clone(),
        response_sender,
    };
    if let Err(error) = injection_writer_sender.send(request).await {
        send_close_from_error("Failed to send request, closing now", error, message_sender).await;
        keep_only_self_running.cancel();
        return;
    }
    let response = match response_receiver.await {
        Ok(response) => response,
        Err(error) => {
            send_close_from_error(
                "Failed to receive response, closing now",
                error,
                message_sender,
            )
            .await;
            keep_only_self_running.cancel();
            return;
        }
    };
    let response = match response {
        Ok(_) => Payload::UnsetInjectedOutputResult {
            id,
            ok: true,
            reason: Default::default(),
        },
        Err(error) => Payload::UnsetInjectedOutputResult {
            id,
            ok: false,
            reason: Some(format!("Failed to update at {:?}: {:?}", path, error)),
        },
    };
    if let Err(error) = message_sender
        .send(Message::Json { payload: response })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
}

async fn handle_binary_message(
    keep_only_self_running: &CancellationToken,
    message_sender: &Sender<Message>,
) {
    warn!("Got binary frame from websocket, closing now");
    if let Err(error) = message_sender
        .send(Message::Close {
            frame: Some(CloseFrame {
                code: CloseCode::Unsupported,
                reason: Cow::from("Unexpected binary frame, closing now"),
            }),
        })
        .await
    {
        error!(
            "Failed to send message into channel for sender: {:?}",
            error
        );
    }
    keep_only_self_running.cancel();
}
