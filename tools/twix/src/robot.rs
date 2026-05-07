use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use color_eyre::eyre::eyre;
use log::error;
use ros_z::{
    context::{Context, ContextBuilder},
    dynamic::DynamicPayload,
    graph::Graph,
    node::Node,
    parameter::{
        GetNodeParameterTypeInfoResponse, GetNodeParameterValueResponse,
        GetNodeParametersSnapshotResponse, RemoteParameterClient, ResetNodeParameterResponse,
        SetNodeParameterResponse,
    },
    pubsub::Received,
    time::Time,
};
use serde_json::Value;
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime},
    sync::watch,
};

use crate::{
    backend::{
        BackendCapability, BackendConnectionStatus, BackendError, BackendResult,
        ConfigNodeDescriptor, ConfigNodeListState, TopicDescriptor, TopicListState, TwixTime,
    },
    change_buffer::{Change, ChangeBuffer, ChangeBufferHandle},
    dynamic_json::dynamic_payload_to_json,
    value_buffer::{Buffer, BufferHandle, Datum},
};

type ChangeCallback = Arc<dyn Fn() + Send + Sync + 'static>;
const DYNAMIC_SCHEMA_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(10);

struct ConnectedBackend {
    generation: u64,
    context: Arc<Context>,
    node: Arc<Node>,
}

pub struct Robot {
    runtime: Runtime,
    endpoint: Arc<Mutex<String>>,
    current_backend: Arc<Mutex<Option<Arc<ConnectedBackend>>>>,
    backend_tx: watch::Sender<Option<Arc<ConnectedBackend>>>,
    status_tx: watch::Sender<BackendConnectionStatus>,
    callbacks: Arc<Mutex<Vec<ChangeCallback>>>,
    generation: Arc<AtomicU64>,
}

impl Robot {
    pub fn new(endpoint: String, _repository: Option<repository::Repository>) -> Self {
        let runtime = RuntimeBuilder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let (backend_tx, _) = watch::channel(None);
        let (status_tx, _) = watch::channel(BackendConnectionStatus::Disconnected);

        Self {
            runtime,
            endpoint: Arc::new(Mutex::new(endpoint)),
            current_backend: Arc::new(Mutex::new(None)),
            backend_tx,
            status_tx,
            callbacks: Arc::new(Mutex::new(Vec::new())),
            generation: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn connect(&self) {
        if !matches!(
            *self.status_tx.borrow(),
            BackendConnectionStatus::Disconnected
        ) {
            return;
        }

        let endpoint = self.endpoint.lock().unwrap().clone();
        let current_backend = self.current_backend.clone();
        let backend_tx = self.backend_tx.clone();
        let status_tx = self.status_tx.clone();
        let callbacks = self.callbacks.clone();
        let generation_counter = self.generation.clone();
        let generation = generation_counter.fetch_add(1, Ordering::AcqRel) + 1;

        status_tx.send_replace(BackendConnectionStatus::Connecting);
        trigger_callbacks(&callbacks);

        self.runtime.spawn(async move {
            let result = connect_backend(&endpoint, generation).await;
            if generation_counter.load(Ordering::Acquire) != generation {
                return;
            }

            match result {
                Ok(backend) => {
                    let backend = Arc::new(backend);
                    *current_backend.lock().unwrap() = Some(backend.clone());
                    backend_tx.send_replace(Some(backend));
                    status_tx.send_replace(BackendConnectionStatus::Connected);
                }
                Err(error) => {
                    error!("failed to connect ros-z backend: {error}");
                    *current_backend.lock().unwrap() = None;
                    backend_tx.send_replace(None);
                    status_tx.send_replace(BackendConnectionStatus::Disconnected);
                }
            }
            trigger_callbacks(&callbacks);
        });
    }

    pub fn disconnect(&self) {
        self.invalidate_backend();
    }

    pub fn connection_status(&self) -> BackendConnectionStatus {
        *self.status_tx.borrow()
    }

    pub fn set_address(&self, endpoint: String) {
        *self.endpoint.lock().unwrap() = endpoint;
        self.invalidate_backend();
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.lock().unwrap().clone()
    }

    pub fn topic_list_state(&self) -> TopicListState {
        let Some(backend) = self.current_backend.lock().unwrap().clone() else {
            return TopicListState {
                discovering: matches!(
                    self.connection_status(),
                    BackendConnectionStatus::Connecting
                ),
                topics: Vec::new(),
            };
        };

        let mut topics = backend
            .context_graph()
            .get_topic_names_and_types()
            .into_iter()
            .map(|(name, graph_type)| TopicDescriptor { name, graph_type })
            .collect::<Vec<_>>();
        topics.sort_by(|left, right| left.name.cmp(&right.name));

        TopicListState {
            discovering: topics.is_empty(),
            topics,
        }
    }

    pub fn config_node_list_state(&self) -> ConfigNodeListState {
        let Some(backend) = self.current_backend.lock().unwrap().clone() else {
            return ConfigNodeListState {
                discovering: matches!(
                    self.connection_status(),
                    BackendConnectionStatus::Connecting
                ),
                nodes: Vec::new(),
            };
        };

        let services = service_names(backend.context_graph());
        let nodes = config_nodes_from_services(&services);
        ConfigNodeListState {
            discovering: nodes.is_empty(),
            nodes,
        }
    }

    pub fn has_capability(&self, capability: BackendCapability) -> bool {
        matches!(
            capability,
            BackendCapability::TopicDiscovery
                | BackendCapability::DynamicInspection
                | BackendCapability::NodeConfigRead
                | BackendCapability::NodeConfigMetadata
                | BackendCapability::NodeConfigWrite
        )
    }

    pub fn subscribe_json(&self, topic: impl Into<String>) -> BufferHandle<Value> {
        self.subscribe_buffered_json(topic, Duration::ZERO)
    }

    pub fn subscribe_buffered_json(
        &self,
        topic: impl Into<String>,
        history: Duration,
    ) -> BufferHandle<Value> {
        let topic = topic.into();
        let (buffer, handle) = Buffer::new(history);
        let mut backend_rx = self.backend_tx.subscribe();
        let callbacks = self.callbacks.clone();

        self.runtime.spawn(async move {
            subscribe_dynamic_json_loop(topic, buffer, &mut backend_rx, callbacks).await;
        });

        handle
    }

    pub fn subscribe_changes_json(&self, topic: impl Into<String>) -> ChangeBufferHandle<Value> {
        let topic = topic.into();
        let (buffer, handle) = ChangeBuffer::new();
        let mut backend_rx = self.backend_tx.subscribe();
        let callbacks = self.callbacks.clone();

        self.runtime.spawn(async move {
            subscribe_dynamic_change_loop(topic, buffer, &mut backend_rx, callbacks).await;
        });

        handle
    }

    pub fn subscribe_value<T>(&self, logical_path: impl Into<String>) -> BufferHandle<T> {
        self.subscribe_buffered_value(logical_path, Duration::ZERO)
    }

    pub fn subscribe_buffered_value<T>(
        &self,
        logical_path: impl Into<String>,
        history: Duration,
    ) -> BufferHandle<T> {
        let logical_path = logical_path.into();
        let (buffer, handle) = Buffer::new(history);
        buffer.push_error(eyre!(BackendError::UnmappedLogicalPath {
            path: logical_path
        }));
        handle
    }

    pub fn subscribe_topic_value<T>(
        &self,
        topic: impl Into<String>,
        history: Duration,
    ) -> BufferHandle<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        let _topic = topic.into();
        let (buffer, handle) = Buffer::new(history);
        buffer.push_error(eyre!(BackendError::UnsupportedCapability {
            operation: "typed.subscribe"
        }));
        handle
    }

    pub fn write(&self, _path: impl Into<String>, _value: Value) -> BackendResult<()> {
        Err(BackendError::UnsupportedCapability { operation: "write" })
    }

    pub fn on_change(&self, callback: impl Fn() + Send + Sync + 'static) {
        self.callbacks.lock().unwrap().push(Arc::new(callback));
    }

    fn invalidate_backend(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        *self.current_backend.lock().unwrap() = None;
        self.backend_tx.send_replace(None);
        self.status_tx
            .send_replace(BackendConnectionStatus::Disconnected);
        trigger_callbacks(&self.callbacks);
    }

    pub fn get_config_snapshot(
        &self,
        selector: &str,
    ) -> BackendResult<GetNodeParametersSnapshotResponse> {
        let client = self.config_client(selector)?;
        self.runtime
            .block_on(client.get_snapshot())
            .map_err(|error| BackendError::Operation {
                operation: "config.get_snapshot",
                message: error.to_string(),
            })
    }

    pub fn get_config_value(
        &self,
        selector: &str,
        path: &str,
    ) -> BackendResult<GetNodeParameterValueResponse> {
        let client = self.config_client(selector)?;
        self.runtime
            .block_on(client.get_value(path))
            .map_err(|error| BackendError::Operation {
                operation: "config.get_value",
                message: error.to_string(),
            })
    }

    pub fn get_config_metadata(
        &self,
        selector: &str,
    ) -> BackendResult<GetNodeParameterTypeInfoResponse> {
        let client = self.config_client(selector)?;
        self.runtime
            .block_on(client.get_type_info())
            .map_err(|error| BackendError::Operation {
                operation: "config.get_type_info",
                message: error.to_string(),
            })
    }

    pub fn set_config_json(
        &self,
        selector: &str,
        path: &str,
        value: &Value,
        layer: String,
        expected_revision: Option<u64>,
    ) -> BackendResult<SetNodeParameterResponse> {
        let client = self.config_client(selector)?;
        self.runtime
            .block_on(client.set_json(path, value, layer, expected_revision))
            .map_err(|error| BackendError::Operation {
                operation: "config.set_json",
                message: error.to_string(),
            })
    }

    pub fn reset_config(
        &self,
        selector: &str,
        path: &str,
        layer: String,
        expected_revision: Option<u64>,
    ) -> BackendResult<ResetNodeParameterResponse> {
        let client = self.config_client(selector)?;
        self.runtime
            .block_on(client.reset(path, layer, expected_revision))
            .map_err(|error| BackendError::Operation {
                operation: "config.reset",
                message: error.to_string(),
            })
    }

    fn config_client(&self, selector: &str) -> BackendResult<RemoteParameterClient> {
        let backend = self
            .current_backend
            .lock()
            .unwrap()
            .clone()
            .ok_or(BackendError::NotConnected)?;
        let services = service_names(backend.context_graph());
        let node_fqn = resolve_config_node_selector(&services, selector)?;
        RemoteParameterClient::new(backend.node.clone(), node_fqn).map_err(|error| {
            BackendError::Operation {
                operation: "config.client",
                message: error.to_string(),
            }
        })
    }
}

impl ConnectedBackend {
    fn context_graph(&self) -> &Graph {
        self.context.graph().as_ref()
    }
}

async fn connect_backend(endpoint: &str, generation: u64) -> color_eyre::Result<ConnectedBackend> {
    let context = Arc::new(
        ContextBuilder::default()
            .with_mode("client")
            .with_connect_endpoints([endpoint.to_string()])
            .build()
            .await
            .map_err(|error| eyre!(error.to_string()))?,
    );
    let node = Arc::new(
        context
            .create_node("twix")
            .with_namespace("tools")
            .build()
            .await
            .map_err(|error| eyre!(error.to_string()))?,
    );
    Ok(ConnectedBackend {
        generation,
        context,
        node,
    })
}

async fn subscribe_dynamic_json_loop(
    topic: String,
    buffer: Buffer<Value, color_eyre::Report>,
    backend_rx: &mut watch::Receiver<Option<Arc<ConnectedBackend>>>,
    callbacks: Arc<Mutex<Vec<ChangeCallback>>>,
) {
    loop {
        if buffer.is_closed() {
            return;
        }
        let backend = tokio::select! {
            backend = wait_for_backend(backend_rx) => backend,
            () = buffer.closed() => return,
        };
        let Some(backend) = backend else {
            return;
        };
        if buffer.is_closed() {
            return;
        }
        let generation = backend.generation;
        let builder_result = tokio::select! {
            builder = backend.node.dynamic_subscriber_auto(&topic, DYNAMIC_SCHEMA_DISCOVERY_TIMEOUT) => {
                builder.map_err(|error| BackendError::Operation {
                    operation: "dynamic.subscribe",
                    message: error.to_string(),
                })
            }
            () = buffer.closed() => return,
        };
        if buffer.is_closed() {
            return;
        }
        if !is_current_backend(backend_rx, generation) {
            continue;
        }
        let builder = match builder_result {
            Ok(builder) => builder,
            Err(error) => {
                if buffer.is_closed() {
                    return;
                }
                if !is_current_backend(backend_rx, generation) {
                    continue;
                }
                buffer.push_error(eyre!(error));
                if buffer.is_closed() {
                    return;
                }
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    () = buffer.closed() => return,
                    changed = backend_rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                continue;
            }
        };
        let subscriber_result = tokio::select! {
            subscriber = builder.build() => {
                subscriber.map_err(|error| BackendError::Operation {
                    operation: "dynamic.subscribe",
                    message: error.to_string(),
                })
            }
            () = buffer.closed() => return,
        };
        if buffer.is_closed() {
            return;
        }
        if !is_current_backend(backend_rx, generation) {
            continue;
        }
        let subscriber = match subscriber_result {
            Ok(subscriber) => subscriber,
            Err(error) => {
                if buffer.is_closed() {
                    return;
                }
                if !is_current_backend(backend_rx, generation) {
                    continue;
                }
                buffer.push_error(eyre!(error));
                if buffer.is_closed() {
                    return;
                }
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    () = buffer.closed() => return,
                    changed = backend_rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                continue;
            }
        };

        loop {
            tokio::select! {
                () = buffer.closed() => return,
                changed = backend_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    if buffer.is_closed() {
                        return;
                    }
                    if backend_rx.borrow().as_ref().map(|backend| backend.generation) != Some(generation) {
                        break;
                    }
                }
                result = subscriber.recv_with_metadata() => {
                    if buffer.is_closed() {
                        return;
                    }
                    if !is_current_backend(backend_rx, generation) {
                        break;
                    }
                    match result {
                        Ok(received) => {
                            if let Some(datum) = dynamic_received_to_datum(received) {
                                if buffer.is_closed() {
                                    return;
                                }
                                buffer.push(datum).await;
                                if buffer.is_closed() {
                                    return;
                                }
                                trigger_callbacks(&callbacks);
                            }
                        }
                        Err(error) => {
                            if buffer.is_closed() {
                                return;
                            }
                            buffer.push_error(eyre!(BackendError::Operation {
                                operation: "dynamic.recv",
                                message: error.to_string(),
                            }));
                            break;
                        }
                    }
                }
            }
        }
    }
}

async fn subscribe_dynamic_change_loop(
    topic: String,
    buffer: ChangeBuffer<Value, color_eyre::Report>,
    backend_rx: &mut watch::Receiver<Option<Arc<ConnectedBackend>>>,
    callbacks: Arc<Mutex<Vec<ChangeCallback>>>,
) {
    loop {
        if buffer.is_closed() {
            return;
        }
        let backend = tokio::select! {
            backend = wait_for_backend(backend_rx) => backend,
            () = buffer.closed() => return,
        };
        let Some(backend) = backend else {
            return;
        };
        if buffer.is_closed() {
            return;
        }
        let generation = backend.generation;
        let builder_result = tokio::select! {
            builder = backend.node.dynamic_subscriber_auto(&topic, DYNAMIC_SCHEMA_DISCOVERY_TIMEOUT) => {
                builder.map_err(|error| BackendError::Operation {
                    operation: "dynamic.subscribe",
                    message: error.to_string(),
                })
            }
            () = buffer.closed() => return,
        };
        if buffer.is_closed() {
            return;
        }
        if !is_current_backend(backend_rx, generation) {
            continue;
        }
        let builder = match builder_result {
            Ok(builder) => builder,
            Err(error) => {
                if buffer.is_closed() {
                    return;
                }
                if !is_current_backend(backend_rx, generation) {
                    continue;
                }
                buffer.push_error(eyre!(error));
                if buffer.is_closed() {
                    return;
                }
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    () = buffer.closed() => return,
                    changed = backend_rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                continue;
            }
        };
        let subscriber_result = tokio::select! {
            subscriber = builder.build() => {
                subscriber.map_err(|error| BackendError::Operation {
                    operation: "dynamic.subscribe",
                    message: error.to_string(),
                })
            }
            () = buffer.closed() => return,
        };
        if buffer.is_closed() {
            return;
        }
        if !is_current_backend(backend_rx, generation) {
            continue;
        }
        let subscriber = match subscriber_result {
            Ok(subscriber) => subscriber,
            Err(error) => {
                if buffer.is_closed() {
                    return;
                }
                if !is_current_backend(backend_rx, generation) {
                    continue;
                }
                buffer.push_error(eyre!(error));
                if buffer.is_closed() {
                    return;
                }
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs(1)) => {}
                    () = buffer.closed() => return,
                    changed = backend_rx.changed() => {
                        if changed.is_err() {
                            return;
                        }
                    }
                }
                continue;
            }
        };

        loop {
            tokio::select! {
                () = buffer.closed() => return,
                changed = backend_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    if buffer.is_closed() {
                        return;
                    }
                    if backend_rx.borrow().as_ref().map(|backend| backend.generation) != Some(generation) {
                        break;
                    }
                }
                result = subscriber.recv_with_metadata() => {
                    if buffer.is_closed() {
                        return;
                    }
                    if !is_current_backend(backend_rx, generation) {
                        break;
                    }
                    match result {
                        Ok(received) => {
                            if let Some(change) = dynamic_received_to_change(received) {
                                if buffer.is_closed() {
                                    return;
                                }
                                buffer.push(change);
                                if buffer.is_closed() {
                                    return;
                                }
                                trigger_callbacks(&callbacks);
                            }
                        }
                        Err(error) => {
                            if buffer.is_closed() {
                                return;
                            }
                            buffer.push_error(eyre!(BackendError::Operation {
                                operation: "dynamic.recv",
                                message: error.to_string(),
                            }));
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn dynamic_received_to_datum(received: Received<DynamicPayload>) -> Option<Datum<Value>> {
    Some(Datum {
        timestamp: received_timestamp(received.transport_time, received.source_time),
        source_timestamp: received.source_time.map(twix_time),
        value: dynamic_payload_to_json(&received.message),
    })
}

fn dynamic_received_to_change(received: Received<DynamicPayload>) -> Option<Change<Value>> {
    Some(Change {
        timestamp: received_timestamp(received.transport_time, received.source_time),
        source_timestamp: received.source_time.map(twix_time),
        value: dynamic_payload_to_json(&received.message),
    })
}

fn received_timestamp(transport_time: Option<Time>, source_time: Option<Time>) -> TwixTime {
    transport_time
        .or(source_time)
        .map(twix_time)
        .or_else(|| TwixTime::from_system_time(SystemTime::now()))
        .unwrap_or_else(|| TwixTime::from_duration(Duration::ZERO))
}

fn twix_time(time: Time) -> TwixTime {
    TwixTime::from_nanos(time.as_nanos())
}

async fn wait_for_backend(
    backend_rx: &mut watch::Receiver<Option<Arc<ConnectedBackend>>>,
) -> Option<Arc<ConnectedBackend>> {
    loop {
        if let Some(backend) = backend_rx.borrow().clone() {
            return Some(backend);
        }
        if backend_rx.changed().await.is_err() {
            return None;
        }
    }
}

fn is_current_backend(
    backend_rx: &watch::Receiver<Option<Arc<ConnectedBackend>>>,
    generation: u64,
) -> bool {
    backend_rx
        .borrow()
        .as_ref()
        .map(|backend| backend.generation)
        == Some(generation)
}

fn trigger_callbacks(callbacks: &Arc<Mutex<Vec<ChangeCallback>>>) {
    let callbacks = callbacks.lock().unwrap().clone();
    for callback in callbacks {
        callback();
    }
}

fn service_names(graph: &Graph) -> BTreeSet<String> {
    graph
        .get_service_names_and_types()
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn config_nodes_from_services(services: &BTreeSet<String>) -> Vec<ConfigNodeDescriptor> {
    const SNAPSHOT_SUFFIX: &str = "/parameter/get_snapshot";
    services
        .iter()
        .filter_map(|service| {
            let node_fqn = service.strip_suffix(SNAPSHOT_SUFFIX)?;
            Some(ConfigNodeDescriptor {
                node_fqn: node_fqn.to_string(),
                metadata_capable: has_config_metadata(services, node_fqn),
            })
        })
        .collect()
}

fn resolve_config_node_selector(
    services: &BTreeSet<String>,
    selector: &str,
) -> BackendResult<String> {
    let nodes = config_nodes_from_services(services)
        .into_iter()
        .map(|node| node.node_fqn)
        .collect::<Vec<_>>();
    if selector.starts_with('/') {
        return nodes
            .into_iter()
            .find(|node_fqn| node_fqn == selector)
            .ok_or_else(|| BackendError::Operation {
                operation: "config.resolve_node",
                message: format!("node not found: {selector}"),
            });
    }

    let matches = nodes
        .into_iter()
        .filter(|node_fqn| {
            node_fqn
                .rsplit('/')
                .next()
                .is_some_and(|name| name == selector)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(BackendError::Operation {
            operation: "config.resolve_node",
            message: format!("node not found: {selector}"),
        }),
        [node_fqn] => Ok(node_fqn.clone()),
        _ => Err(BackendError::Operation {
            operation: "config.resolve_node",
            message: format!(
                "node name '{selector}' is ambiguous: {}",
                matches.join(", ")
            ),
        }),
    }
}

fn has_config_metadata(services: &BTreeSet<String>, node_fqn: &str) -> bool {
    services.contains(&format!("{node_fqn}/parameter/get_type_info"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::config_nodes_from_services;

    #[test]
    fn config_nodes_from_services_uses_remote_parameter_snapshot_services() {
        let services = BTreeSet::from([
            "/motion/walk/parameter/get_snapshot".to_string(),
            "/motion/walk/parameter/get_type_info".to_string(),
            "/legacy/config/get_snapshot".to_string(),
        ]);

        let nodes = config_nodes_from_services(&services);

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_fqn, "/motion/walk");
        assert!(nodes[0].metadata_capable);
    }
}
