pub mod catalog;
pub(crate) mod connection;
pub mod retained_subscription;
pub mod subscription;
pub mod topic;

use std::sync::Arc;

use color_eyre::{Result, eyre::eyre};
use eframe::egui::Context as EguiContext;
use log::error;
use parking_lot::Mutex;
use ros_z::{context::ContextBuilder, graph::GraphChangeSubscription, node::Node};
use ros_z_debug::RetentionPolicy;
use tokio::{
    runtime::{Builder, Runtime},
    sync::watch,
};

use crate::backend::{
    catalog::TopicCatalog,
    connection::{ConnectionState, ConnectionStatus},
};

pub struct TwixBackend {
    router_endpoint_sender: watch::Sender<String>,
    keep_connected_sender: watch::Sender<bool>,
    connection_state_receiver: watch::Receiver<ConnectionState>,
    target_namespace_sender: watch::Sender<String>,
    topic_catalog: Arc<Mutex<Arc<TopicCatalog>>>,
    egui_context: EguiContext,
    runtime: Runtime,
}

impl TwixBackend {
    pub fn validate_router_endpoint(router_endpoint: impl AsRef<str>) -> Result<()> {
        let router_endpoint = router_endpoint.as_ref();
        if router_endpoint.trim().is_empty() {
            return Err(eyre!("router endpoint must not be empty"));
        }

        ContextBuilder::default().with_router_endpoint(router_endpoint.to_string())?;
        Ok(())
    }

    pub fn new(
        router_endpoint: impl Into<String>,
        target_namespace: impl AsRef<str>,
        egui_context: EguiContext,
    ) -> Result<Self> {
        Self::new_with_keep_connected(router_endpoint, target_namespace, egui_context, true)
    }

    pub fn new_with_keep_connected(
        router_endpoint: impl Into<String>,
        target_namespace: impl AsRef<str>,
        egui_context: EguiContext,
        keep_connected: bool,
    ) -> Result<Self> {
        let target_namespace = topic::normalize_namespace(target_namespace.as_ref())?;
        let router_endpoint = router_endpoint.into();
        Self::validate_router_endpoint(&router_endpoint)?;
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        let (router_endpoint_sender, router_endpoint_receiver) =
            watch::channel(router_endpoint.clone());
        let (keep_connected_sender, keep_connected_receiver) = watch::channel(keep_connected);
        let initial_connection_state = if keep_connected {
            ConnectionState::connecting(router_endpoint)
        } else {
            ConnectionState::disconnected()
        };
        let (connection_state_sender, connection_state_receiver) =
            watch::channel(initial_connection_state);
        let (target_namespace_sender, target_namespace_receiver) = watch::channel(target_namespace);
        let topic_catalog = Arc::new(Mutex::new(Arc::new(TopicCatalog::default())));
        let backend_egui_context = egui_context.clone();

        connection::spawn_connection_task(
            &runtime,
            router_endpoint_receiver,
            keep_connected_receiver,
            connection_state_sender,
            egui_context.clone(),
        );
        spawn_catalog_task(
            &runtime,
            connection_state_receiver.clone(),
            target_namespace_receiver,
            topic_catalog.clone(),
            egui_context,
        );

        Ok(Self {
            router_endpoint_sender,
            keep_connected_sender,
            connection_state_receiver,
            target_namespace_sender,
            topic_catalog,
            egui_context: backend_egui_context,
            runtime,
        })
    }

    pub fn target_namespace(&self) -> String {
        self.target_namespace_sender.borrow().clone()
    }

    pub fn router_endpoint(&self) -> String {
        self.router_endpoint_sender.borrow().clone()
    }

    pub fn set_router_endpoint(&self, router_endpoint: impl Into<String>) -> Result<()> {
        let router_endpoint = router_endpoint.into();
        Self::validate_router_endpoint(&router_endpoint)?;
        if self.router_endpoint() != router_endpoint {
            self.router_endpoint_sender.send_replace(router_endpoint);
            self.egui_context.request_repaint();
        }
        Ok(())
    }

    pub fn keep_connected(&self) -> bool {
        *self.keep_connected_sender.borrow()
    }

    pub fn set_keep_connected(&self, keep_connected: bool) {
        if self.keep_connected() != keep_connected {
            self.keep_connected_sender.send_replace(keep_connected);
            self.egui_context.request_repaint();
        }
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        self.connection_state_receiver.borrow().status()
    }

    pub fn connection_unavailable_message(&self) -> Option<String> {
        self.connection_state_receiver
            .borrow()
            .unavailable_message()
            .map(ToOwned::to_owned)
    }

    pub fn set_target_namespace(&self, target_namespace: impl AsRef<str>) -> Result<()> {
        let target_namespace = topic::normalize_namespace(target_namespace.as_ref())?;
        if self.target_namespace() != target_namespace {
            self.target_namespace_sender.send_replace(target_namespace);
        }
        Ok(())
    }

    pub fn topic_catalog(&self) -> Arc<TopicCatalog> {
        self.topic_catalog.lock().clone()
    }

    pub fn subscribe_json_retained(
        &self,
        selector: impl Into<String>,
        retention: RetentionPolicy,
    ) -> retained_subscription::DynamicSubscription {
        retained_subscription::subscribe_dynamic(
            &self.runtime,
            self.connection_state_receiver.clone(),
            self.target_namespace_sender.subscribe(),
            self.egui_context.clone(),
            selector,
            retention,
        )
    }
}

fn spawn_catalog_task(
    runtime: &Runtime,
    mut connection_state: watch::Receiver<ConnectionState>,
    mut target_namespace: watch::Receiver<String>,
    topic_catalog: Arc<Mutex<Arc<TopicCatalog>>>,
    egui_context: EguiContext,
) {
    runtime.spawn(async move {
        let mut current_node = None;
        let mut graph_changes = None;
        rebuild_or_clear_topic_catalog(
            &mut connection_state,
            &mut target_namespace,
            &topic_catalog,
            &egui_context,
            &mut current_node,
            &mut graph_changes,
        );

        loop {
            if let Some(graph_change_receiver) = graph_changes.as_mut() {
                tokio::select! {
                    changed = graph_change_receiver.changed() => {
                        if changed.is_none() {
                            current_node = None;
                            graph_changes = None;
                            clear_topic_catalog(&topic_catalog, &egui_context);
                        } else if let Some(node) = &current_node {
                            rebuild_topic_catalog(
                                node,
                                &mut target_namespace,
                                &topic_catalog,
                                &egui_context,
                            );
                        }
                    }
                    changed = connection_state.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        rebuild_or_clear_topic_catalog(
                            &mut connection_state,
                            &mut target_namespace,
                            &topic_catalog,
                            &egui_context,
                            &mut current_node,
                            &mut graph_changes,
                        );
                    }
                    changed = target_namespace.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        if let Some(node) = &current_node {
                            rebuild_topic_catalog(
                                node,
                                &mut target_namespace,
                                &topic_catalog,
                                &egui_context,
                            );
                        }
                    }
                }
            } else {
                tokio::select! {
                    changed = connection_state.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        rebuild_or_clear_topic_catalog(
                            &mut connection_state,
                            &mut target_namespace,
                            &topic_catalog,
                            &egui_context,
                            &mut current_node,
                            &mut graph_changes,
                        );
                    }
                    changed = target_namespace.changed() => {
                        if changed.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });
}

fn rebuild_or_clear_topic_catalog(
    connection_state: &mut watch::Receiver<ConnectionState>,
    target_namespace: &mut watch::Receiver<String>,
    topic_catalog: &Mutex<Arc<TopicCatalog>>,
    egui_context: &EguiContext,
    current_node: &mut Option<Arc<Node>>,
    graph_changes: &mut Option<GraphChangeSubscription>,
) {
    let state = connection_state.borrow_and_update().clone();
    if let Some(node) = state.node() {
        *graph_changes = Some(node.graph().subscribe_changes());
        rebuild_topic_catalog(&node, target_namespace, topic_catalog, egui_context);
        *current_node = Some(node);
    } else {
        *current_node = None;
        *graph_changes = None;
        clear_topic_catalog(topic_catalog, egui_context);
    }
}

fn clear_topic_catalog(topic_catalog: &Mutex<Arc<TopicCatalog>>, egui_context: &EguiContext) {
    *topic_catalog.lock() = Arc::new(TopicCatalog::default());
    egui_context.request_repaint();
}

fn rebuild_topic_catalog(
    node: &Node,
    target_namespace: &mut watch::Receiver<String>,
    topic_catalog: &Mutex<Arc<TopicCatalog>>,
    egui_context: &EguiContext,
) {
    let target_namespace = target_namespace.borrow_and_update().clone();
    match catalog::build_topic_catalog(&target_namespace, &node.graph().view()) {
        Ok(catalog) => {
            *topic_catalog.lock() = Arc::new(catalog);
            egui_context.request_repaint();
        }
        Err(error) => error!("failed to rebuild Twix topic catalog: {error:#}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::connection::{ConnectionState, ConnectionStatus};

    use super::*;

    #[test]
    fn disconnected_connection_state_reports_unavailable_message() {
        let state = ConnectionState::disconnected();

        assert_eq!(state.status(), ConnectionStatus::Disconnected);
        assert_eq!(state.unavailable_message(), Some("Twix is disconnected"));
    }

    #[test]
    fn failed_connection_state_reports_router_error() {
        let state = ConnectionState::failed(
            "tcp/127.0.0.1:7447".to_string(),
            "router refused connection".to_string(),
        );

        assert_eq!(state.status(), ConnectionStatus::Failed);
        assert_eq!(
            state.unavailable_message(),
            Some("Twix connection to tcp/127.0.0.1:7447 failed: router refused connection")
        );
    }

    #[test]
    fn connecting_connection_state_reports_router_endpoint() {
        let state = ConnectionState::connecting("tcp/127.0.0.1:7447".to_string());

        assert_eq!(state.status(), ConnectionStatus::Connecting);
        assert_eq!(
            state.unavailable_message(),
            Some("Twix is connecting to tcp/127.0.0.1:7447")
        );
    }

    #[test]
    fn backend_can_start_intentionally_disconnected() {
        let backend = TwixBackend::new_with_keep_connected(
            "tcp/127.0.0.1:7447",
            "/42",
            EguiContext::default(),
            false,
        )
        .unwrap();

        assert!(!backend.keep_connected());
        assert_eq!(backend.connection_status(), ConnectionStatus::Disconnected);
        assert_eq!(backend.router_endpoint(), "tcp/127.0.0.1:7447");
    }

    #[test]
    fn backend_updates_router_endpoint_without_recreating_backend() {
        let backend = TwixBackend::new_with_keep_connected(
            "tcp/127.0.0.1:7447",
            "/42",
            EguiContext::default(),
            false,
        )
        .unwrap();

        backend.set_router_endpoint("tcp/127.0.0.1:7448").unwrap();

        assert_eq!(backend.router_endpoint(), "tcp/127.0.0.1:7448");
        assert_eq!(backend.connection_status(), ConnectionStatus::Disconnected);
    }
}
