use std::{sync::Arc, time::Duration};

use color_eyre::{Result, eyre::Context};
use eframe::egui::Context as EguiContext;
use ros_z::{context::ContextBuilder, node::Node};
use tokio::{runtime::Runtime, sync::watch, time};

const CONNECTION_RETRY_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

#[derive(Clone)]
pub(crate) enum ConnectionState {
    Disconnected,
    Connecting { message: Arc<str> },
    Connected { node: Arc<Node> },
    Failed { message: Arc<str> },
}

impl ConnectionState {
    pub(crate) fn disconnected() -> Self {
        Self::Disconnected
    }

    pub(crate) fn connecting(router_endpoint: String) -> Self {
        let message = format!("Twix is connecting to {router_endpoint}");
        Self::Connecting {
            message: string_to_arc_str(message),
        }
    }

    pub(crate) fn connected(node: Arc<Node>) -> Self {
        Self::Connected { node }
    }

    pub(crate) fn failed(router_endpoint: String, message: String) -> Self {
        let formatted_message = format!("Twix connection to {router_endpoint} failed: {message}");
        Self::Failed {
            message: string_to_arc_str(formatted_message),
        }
    }

    pub(crate) fn status(&self) -> ConnectionStatus {
        match self {
            Self::Disconnected => ConnectionStatus::Disconnected,
            Self::Connecting { .. } => ConnectionStatus::Connecting,
            Self::Connected { .. } => ConnectionStatus::Connected,
            Self::Failed { .. } => ConnectionStatus::Failed,
        }
    }

    pub(crate) fn node(&self) -> Option<Arc<Node>> {
        match self {
            Self::Connected { node, .. } => Some(node.clone()),
            _ => None,
        }
    }

    pub(crate) fn unavailable_message(&self) -> Option<&str> {
        match self {
            Self::Disconnected => Some("Twix is disconnected"),
            Self::Connecting { message } | Self::Failed { message } => Some(message),
            Self::Connected { .. } => None,
        }
    }
}

pub(crate) fn spawn_connection_task(
    runtime: &Runtime,
    router_endpoint: watch::Receiver<String>,
    keep_connected: watch::Receiver<bool>,
    connection_state: watch::Sender<ConnectionState>,
    egui_context: EguiContext,
) {
    runtime.spawn(run_connection_task(
        router_endpoint,
        keep_connected,
        connection_state,
        egui_context,
    ));
}

async fn run_connection_task(
    mut router_endpoint: watch::Receiver<String>,
    mut keep_connected: watch::Receiver<bool>,
    connection_state: watch::Sender<ConnectionState>,
    egui_context: EguiContext,
) {
    loop {
        if !*keep_connected.borrow_and_update() {
            connection_state.send_replace(ConnectionState::disconnected());
            egui_context.request_repaint();

            tokio::select! {
                changed = keep_connected.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                changed = router_endpoint.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }
            continue;
        }

        let endpoint = router_endpoint.borrow_and_update().clone();
        connection_state.send_replace(ConnectionState::connecting(endpoint.clone()));
        egui_context.request_repaint();

        let connect = connect(endpoint.clone());
        tokio::pin!(connect);
        let result = tokio::select! {
            result = &mut connect => result,
            changed = keep_connected.changed() => {
                if changed.is_err() {
                    break;
                }
                continue;
            }
            changed = router_endpoint.changed() => {
                if changed.is_err() {
                    break;
                }
                continue;
            }
        };

        match result {
            Ok(node) => {
                connection_state.send_replace(ConnectionState::connected(node));
                egui_context.request_repaint();

                tokio::select! {
                    changed = keep_connected.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    changed = router_endpoint.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                connection_state
                    .send_replace(ConnectionState::failed(endpoint, format!("{error:#}")));
                egui_context.request_repaint();

                let retry = time::sleep(CONNECTION_RETRY_DELAY);
                tokio::pin!(retry);
                tokio::select! {
                    _ = &mut retry => {}
                    changed = keep_connected.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                    changed = router_endpoint.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn connect(router_endpoint: String) -> Result<Arc<Node>> {
    let context = ContextBuilder::default()
        .with_router_endpoint(router_endpoint)
        .wrap_err("failed to configure Twix router endpoint")?
        .build()
        .await
        .wrap_err("failed to create Twix ros-z context")?;
    let node = context
        .create_node("twix")
        .build()
        .await
        .wrap_err("failed to create Twix ros-z node")?;
    Ok(Arc::new(node))
}

fn string_to_arc_str(value: String) -> Arc<str> {
    Arc::from(value.into_boxed_str())
}
