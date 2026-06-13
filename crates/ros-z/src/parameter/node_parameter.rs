use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use arc_swap::ArcSwap;
use parking_lot::Mutex;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::watch;
use tokio_util::task::AbortOnDropHandle;

use crate::{
    Message, entity::SchemaHash, message::validated_type_info_for_schema, node::Node,
    pubsub::Publisher, time::Clock,
};

use super::{
    FieldPath, LayerPath, NodeParametersSnapshot, ParameterError, ParameterKey,
    ParameterSubscription, ParameterTimestamp, Result,
    loader::load_json5_object_or_empty,
    merge::{
        RecursiveDiffEntry, get_value_at_path as get_from_value, merge_layers, provenance_for_path,
        recursive_diff, remove_value_at_path, set_value_at_path,
    },
    persistence::write_pretty_json_batch,
    remote::{
        services::{self, RemoteParameterServices},
        types::{NodeParameterChange, NodeParameterChangeSource, NodeParameterEvent},
    },
};

type ValidateHook<T> = Arc<dyn Fn(&T) -> std::result::Result<(), String> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ParameterJsonWrite {
    pub path: FieldPath,
    pub value: Value,
    pub target_layer: LayerPath,
}

#[derive(Clone)]
pub struct NodeParameters<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    inner: Arc<NodeParametersInner<T>>,
}

struct NodeParametersInner<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    // Field order is shutdown order: stop accepting remote requests, close the
    // command sender, abort the driver if it is still running, then release the
    // node-level binding guard.
    _remote: RemoteParameterServices<T>,
    commands: flume::Sender<ParameterCommand<T>>,
    driver_task: AbortOnDropHandle<()>,
    _binding_guard: BindingGuard,
    state: Arc<ParameterState<T>>,
}

struct BindingGuard {
    state: Arc<parking_lot::Mutex<bool>>,
}

impl Drop for BindingGuard {
    fn drop(&mut self) {
        *self.state.lock() = false;
    }
}

pub struct ParameterState<T> {
    node_fqn: String,
    parameter_key: ParameterKey,
    pub type_name: String,
    pub schema_hash: SchemaHash,
    layers: Vec<PathBuf>,
    clock: Clock,
    commit_lock: Mutex<()>,
    hooks: Mutex<Vec<ValidateHook<T>>>,
    pub current: ArcSwap<NodeParametersSnapshot<T>>,
    tx: watch::Sender<Arc<NodeParametersSnapshot<T>>>,
}

impl<T> std::ops::Deref for NodeParametersInner<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    type Target = ParameterState<T>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

const PARAMETER_MAILBOX_CAPACITY: usize = 64;

type ParameterReply<T> = flume::Sender<Result<T>>;

pub enum ParameterCommand<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    SetJson {
        writes: Vec<ParameterJsonWrite>,
        expected_revision: Option<u64>,
        source: NodeParameterChangeSource,
        reply: ParameterReply<CommitOutcome>,
    },
    Reset {
        resets: Vec<(FieldPath, LayerPath)>,
        expected_revision: Option<u64>,
        source: NodeParameterChangeSource,
        reply: ParameterReply<CommitOutcome>,
    },
    Reload {
        source: NodeParameterChangeSource,
        reply: ParameterReply<CommitOutcome>,
    },
    AddValidationHook {
        hook: ValidateHook<T>,
        reply: ParameterReply<()>,
    },
    RemoteGetSnapshot {
        query: zenoh::query::Query,
    },
    RemoteGetValue {
        query: zenoh::query::Query,
    },
    RemoteGetTypeInfo {
        query: zenoh::query::Query,
    },
    RemoteSet {
        query: zenoh::query::Query,
    },
    RemoteSetAtomic {
        query: zenoh::query::Query,
    },
    RemoteReset {
        query: zenoh::query::Query,
    },
    RemoteReload {
        query: zenoh::query::Query,
    },
}

pub struct ParameterDriver<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    state: Arc<ParameterState<T>>,
    commands: flume::Receiver<ParameterCommand<T>>,
    event_publisher: Arc<Publisher<NodeParameterEvent>>,
}

fn actor_unavailable_error() -> ParameterError {
    ParameterError::RemoteError {
        message: "parameter actor is unavailable".to_string(),
    }
}

impl Node {
    /// Binds this node to its typed parameter set and starts the parameter actor.
    ///
    /// The returned handle reads from the current snapshot immediately and sends
    /// mutations through the actor. Remote parameter services are registered as
    /// part of the binding, so remote reads and writes are serialized through the
    /// same actor as local writes.
    ///
    /// A node can only have one active parameter binding. Calling this again on
    /// the same node while the first [`NodeParameters`] handle is alive returns
    /// [`ParameterError::AlreadyBound`]. Dropping the handle shuts down the actor
    /// and releases the binding.
    ///
    /// # Errors
    ///
    /// Returns an error if the key is invalid, no parameter layers are configured,
    /// the initial JSON5 layers cannot be loaded or deserialized as `T`, schema or
    /// remote-service registration fails, or the node already has an active
    /// parameter binding.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ros_z::{context::ContextBuilder, node::Node, parameter::NodeParameters};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
    /// #[message(name = "example::DetectorParameters")]
    /// struct DetectorParameters {
    ///     enabled: bool,
    /// }
    ///
    /// async fn bind(node: &Node) -> ros_z::parameter::Result<NodeParameters<DetectorParameters>> {
    ///     node.bind_parameter_as::<DetectorParameters>("detector").await
    /// }
    ///
    /// # async fn build_node() -> ros_z::Result<Node> {
    /// #     let context = ContextBuilder::default()
    /// #         .with_parameter_layers([std::path::PathBuf::from("etc/parameters/ros_z/base")])
    /// #         .build()
    /// #         .await?;
    /// #     context.create_node("detector").build().await
    /// # }
    /// ```
    pub async fn bind_parameter_as<T>(
        &self,
        parameter_key: impl Into<ParameterKey>,
    ) -> Result<NodeParameters<T>>
    where
        T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
    {
        let parameter_key = parameter_key.into();
        validate_parameter_key(&parameter_key)?;
        mark_parameter_bound(self)?;

        match build_parameter_actor(self, parameter_key).await {
            Ok(parameters) => Ok(parameters),
            Err(err) => {
                clear_parameter_bound(self);
                Err(err)
            }
        }
    }
}

fn mark_parameter_bound(node: &Node) -> Result<()> {
    let mut bound = node.parameter_binding_state().lock();
    if *bound {
        return Err(ParameterError::AlreadyBound {
            node_fqn: node.node_entity().fully_qualified_name(),
        });
    }
    *bound = true;
    Ok(())
}

fn clear_parameter_bound(node: &Node) {
    *node.parameter_binding_state().lock() = false;
}

async fn build_parameter_actor<T>(
    node: &Node,
    parameter_key: ParameterKey,
) -> Result<NodeParameters<T>>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let layers = node.runtime_parameter_inputs().parameter_layers.clone();
    if layers.is_empty() {
        return Err(ParameterError::EmptyLayerList);
    }

    let schema = Arc::new(T::schema());
    let type_info = validated_type_info_for_schema::<T>(&schema);
    node.register_schema_with_service(&type_info.name, schema)
        .map_err(|source| ParameterError::operation("registering parameter schema", source))?;

    let node_fqn = node.node_entity().fully_qualified_name();
    let snapshot = load_snapshot::<T>(&node_fqn, &parameter_key, &layers, node.clock(), 0)?;
    let snapshot = Arc::new(snapshot);
    let (tx, _rx) = watch::channel(snapshot.clone());

    let current = ArcSwap::from(snapshot);
    let state = Arc::new(ParameterState {
        node_fqn: node_fqn.clone(),
        parameter_key,
        type_name: type_info.name,
        schema_hash: type_info.hash,
        layers,
        clock: node.clock().clone(),
        commit_lock: Mutex::new(()),
        hooks: Mutex::new(Vec::new()),
        current,
        tx,
    });
    let (command_tx, command_rx) = flume::bounded(PARAMETER_MAILBOX_CAPACITY);
    let remote = RemoteParameterServices::register(node, state.clone(), command_tx.clone()).await?;
    let event_publisher = remote.event_publisher();

    let driver = ParameterDriver {
        state: state.clone(),
        commands: command_rx,
        event_publisher,
    };
    let driver_task = tokio::spawn(driver.run());
    let inner = Arc::new(NodeParametersInner {
        _remote: remote,
        commands: command_tx,
        driver_task: AbortOnDropHandle::new(driver_task),
        _binding_guard: BindingGuard {
            state: node.parameter_binding_state().clone(),
        },
        state,
    });

    Ok(NodeParameters { inner })
}

fn validate_parameter_key(parameter_key: &str) -> Result<()> {
    if parameter_key.is_empty()
        || !parameter_key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(ParameterError::InvalidParameterKey {
            key: parameter_key.to_string(),
        });
    }

    Ok(())
}

fn layer_path(path: &std::path::Path) -> LayerPath {
    path.to_string_lossy().into_owned()
}

fn load_snapshot<T>(
    node_fqn: &str,
    parameter_key: &str,
    layers: &[PathBuf],
    clock: &Clock,
    revision: u64,
) -> Result<NodeParametersSnapshot<T>>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let layer_overlays = layers
        .iter()
        .map(|layer| load_json5_object_or_empty(&layer.join(format!("{parameter_key}.json5"))))
        .collect::<Result<Vec<_>>>()?;

    snapshot_from_parts(
        node_fqn,
        parameter_key,
        layers,
        clock,
        revision,
        layer_overlays,
    )
}

fn snapshot_from_parts<T>(
    node_fqn: &str,
    parameter_key: &str,
    layers: &[PathBuf],
    clock: &Clock,
    revision: u64,
    layer_overlays: Vec<Value>,
) -> Result<NodeParametersSnapshot<T>>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    let layers = layers
        .iter()
        .map(|path| layer_path(path))
        .collect::<Vec<_>>();
    let merge_inputs = layers
        .iter()
        .zip(layer_overlays.iter())
        .map(|(layer, overlay)| (layer.as_str(), overlay))
        .collect::<Vec<_>>();
    let merged = merge_layers(&merge_inputs)?;
    let typed: T = serde_json::from_value(merged.effective.clone())
        .map_err(|err| ParameterError::DeserializationError { source: err })?;

    Ok(NodeParametersSnapshot {
        node_fqn: node_fqn.to_string(),
        parameter_key: parameter_key.to_string(),
        typed: Arc::new(typed),
        effective: merged.effective,
        layers,
        layer_overlays,
        provenance: Arc::new(merged.provenance),
        revision,
        committed_at: ParameterTimestamp::now_from(clock),
    })
}

impl<T> NodeParameters<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    async fn request<R>(
        &self,
        command: impl FnOnce(ParameterReply<R>) -> ParameterCommand<T>,
    ) -> Result<R>
    where
        R: Send + 'static,
    {
        let (reply_tx, reply_rx) = flume::bounded(1);
        let commands = self.command_sender()?;
        commands
            .send_async(command(reply_tx))
            .await
            .map_err(|_| actor_unavailable_error())?;
        tokio::select! {
            result = reply_rx.recv_async() => result.map_err(|_| actor_unavailable_error())?,
            () = self.wait_for_driver_task_to_finish() => Err(actor_unavailable_error()),
        }
    }

    fn command_sender(&self) -> Result<flume::Sender<ParameterCommand<T>>> {
        if self.driver_task_is_unavailable() {
            return Err(actor_unavailable_error());
        }

        Ok(self.inner.commands.clone())
    }

    async fn wait_for_driver_task_to_finish(&self) {
        while !self.driver_task_is_unavailable() {
            tokio::task::yield_now().await;
        }
    }

    fn driver_task_is_unavailable(&self) -> bool {
        self.inner.driver_task.is_finished()
    }

    pub fn snapshot(&self) -> Arc<NodeParametersSnapshot<T>> {
        self.inner.state.current.load_full()
    }

    pub fn get_json(&self, path: &str) -> Result<Value> {
        get_from_value(&self.snapshot().effective, path)?.ok_or_else(|| ParameterError::PathError {
            path: path.to_string(),
            reason: "path not found".to_string(),
        })
    }

    pub async fn set_json(
        &self,
        path: &str,
        value: Value,
        target_layer: impl Into<LayerPath>,
    ) -> Result<()> {
        self.set_json_atomically(
            vec![ParameterJsonWrite {
                path: path.to_string(),
                value,
                target_layer: target_layer.into(),
            }],
            None,
        )
        .await
        .map(|_| ())
    }

    pub async fn set_json_atomically(
        &self,
        changes: Vec<ParameterJsonWrite>,
        expected_revision: Option<u64>,
    ) -> Result<CommitOutcome> {
        self.request(|reply| ParameterCommand::SetJson {
            writes: changes,
            expected_revision,
            source: NodeParameterChangeSource::LocalWrite,
            reply,
        })
        .await
    }

    pub async fn reset(&self, path: &str, target_layer: impl Into<LayerPath>) -> Result<()> {
        self.request(|reply| ParameterCommand::Reset {
            resets: vec![(path.to_string(), target_layer.into())],
            expected_revision: None,
            source: NodeParameterChangeSource::LocalWrite,
            reply,
        })
        .await
        .map(|_| ())
    }

    pub async fn reload(&self) -> Result<()> {
        self.request(|reply| ParameterCommand::Reload {
            source: NodeParameterChangeSource::Reload,
            reply,
        })
        .await
        .map(|_| ())
    }

    pub fn subscribe(&self) -> ParameterSubscription<T> {
        self.inner.state.tx.subscribe()
    }

    pub async fn add_validation_hook<F>(&self, hook: F) -> Result<()>
    where
        F: Fn(&T) -> std::result::Result<(), String> + Send + Sync + 'static,
    {
        let hook: ValidateHook<T> = Arc::new(hook);
        self.request(|reply| ParameterCommand::AddValidationHook { hook, reply })
            .await
    }
}

impl<T> ParameterCommand<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    pub fn into_query(self) -> zenoh::query::Query {
        match self {
            ParameterCommand::RemoteGetSnapshot { query }
            | ParameterCommand::RemoteGetValue { query }
            | ParameterCommand::RemoteGetTypeInfo { query }
            | ParameterCommand::RemoteSet { query }
            | ParameterCommand::RemoteSetAtomic { query }
            | ParameterCommand::RemoteReset { query }
            | ParameterCommand::RemoteReload { query } => query,
            ParameterCommand::SetJson { .. }
            | ParameterCommand::Reset { .. }
            | ParameterCommand::Reload { .. }
            | ParameterCommand::AddValidationHook { .. } => {
                unreachable!("local parameter commands do not carry remote queries")
            }
        }
    }
}

impl<T> ParameterDriver<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    async fn run(self) {
        while let Ok(command) = self.commands.recv_async().await {
            match command {
                ParameterCommand::SetJson {
                    writes,
                    expected_revision,
                    source,
                    reply,
                } => {
                    let result = self.commit(&writes, &[], expected_revision, source).await;
                    let _ = reply.send(result);
                }
                ParameterCommand::Reset {
                    resets,
                    expected_revision,
                    source,
                    reply,
                } => {
                    let result = self.commit(&[], &resets, expected_revision, source).await;
                    let _ = reply.send(result);
                }
                ParameterCommand::Reload { source, reply } => {
                    let result = self.reload_with_source(source).await;
                    let _ = reply.send(result);
                }
                ParameterCommand::AddValidationHook { hook, reply } => {
                    let result = self.add_validation_hook(hook);
                    let _ = reply.send(result);
                }
                remote_command => self.handle_remote_command(remote_command).await,
            }
        }
    }

    async fn handle_remote_command(&self, command: ParameterCommand<T>) {
        match command {
            ParameterCommand::RemoteGetSnapshot { query } => {
                services::handle_get_snapshot_for_state(&self.state, query);
            }
            ParameterCommand::RemoteGetValue { query } => {
                services::handle_get_value_for_state(&self.state, query);
            }
            ParameterCommand::RemoteGetTypeInfo { query } => {
                services::handle_get_type_info_for_state(&self.state, query);
            }
            ParameterCommand::RemoteSet { query } => {
                services::handle_set_for_driver(self, query).await;
            }
            ParameterCommand::RemoteSetAtomic { query } => {
                services::handle_set_atomic_for_driver(self, query).await;
            }
            ParameterCommand::RemoteReset { query } => {
                services::handle_reset_for_driver(self, query).await;
            }
            ParameterCommand::RemoteReload { query } => {
                services::handle_reload_for_driver(self, query).await;
            }
            ParameterCommand::SetJson { .. }
            | ParameterCommand::Reset { .. }
            | ParameterCommand::Reload { .. }
            | ParameterCommand::AddValidationHook { .. } => {
                unreachable!("local parameter commands are handled before remote dispatch")
            }
        }
    }

    pub async fn reload_with_source(
        &self,
        source: NodeParameterChangeSource,
    ) -> Result<CommitOutcome> {
        let (outcome, event) = {
            let _commit_guard = self.state.commit_lock.lock();
            let current = self.state.current.load_full();

            let candidate = load_snapshot::<T>(
                &self.state.node_fqn,
                &self.state.parameter_key,
                &self.state.layers,
                &self.state.clock,
                current.revision + 1,
            )?;
            self.run_hooks(candidate.typed.as_ref())?;
            let diff = recursive_diff(&current.effective, &candidate.effective);
            let changed_paths = diff.iter().map(|entry| entry.path.clone()).collect();
            let snapshot = Arc::new(candidate);
            self.state.current.store(snapshot.clone());
            let _ = self.state.tx.send(snapshot.clone());
            let event = self.build_event(&current, &snapshot, diff, source);
            (
                CommitOutcome {
                    committed_revision: snapshot.revision,
                    changed_paths,
                },
                event,
            )
        };
        if let Err(err) = self.publish_event(&event).await {
            tracing::warn!("[PARAM] Failed to publish parameter event: {err}");
        }
        Ok(outcome)
    }

    fn add_validation_hook(&self, hook: ValidateHook<T>) -> Result<()> {
        let _commit_guard = self.state.commit_lock.lock();
        self.run_hook(self.state.current.load_full().typed.as_ref(), &hook)?;
        self.state.hooks.lock().push(hook);
        Ok(())
    }

    fn run_hooks(&self, candidate: &T) -> Result<()> {
        for hook in self.state.hooks.lock().iter() {
            self.run_hook(candidate, hook)?;
        }
        Ok(())
    }

    fn run_hook(&self, candidate: &T, hook: &ValidateHook<T>) -> Result<()> {
        hook(candidate).map_err(|message| ParameterError::ValidationError { message })
    }

    pub async fn commit(
        &self,
        writes: &[ParameterJsonWrite],
        resets: &[(FieldPath, LayerPath)],
        expected_revision: Option<u64>,
        source: NodeParameterChangeSource,
    ) -> Result<CommitOutcome> {
        let (outcome, event) = self.commit_state(writes, resets, expected_revision, source)?;
        if let Err(err) = self.publish_event(&event).await {
            tracing::warn!("[PARAM] Failed to publish parameter event: {err}");
        }
        Ok(outcome)
    }

    fn commit_state(
        &self,
        writes: &[ParameterJsonWrite],
        resets: &[(FieldPath, LayerPath)],
        expected_revision: Option<u64>,
        source: NodeParameterChangeSource,
    ) -> Result<(CommitOutcome, NodeParameterEvent)> {
        let _commit_guard = self.state.commit_lock.lock();
        let current = self.state.current.load_full();
        if let Some(expected) = expected_revision
            && expected != current.revision
        {
            return Err(ParameterError::RevisionMismatch {
                expected,
                actual: current.revision,
            });
        }

        let mut layer_overlays = current.layer_overlays.clone();
        let active_layers = &current.layers;
        let mut touched = BTreeSet::new();

        for write in writes {
            let index = active_layers
                .iter()
                .position(|layer| layer == &write.target_layer)
                .ok_or_else(|| ParameterError::LayerNotActive {
                    layer: write.target_layer.clone(),
                })?;
            let overlay = &mut layer_overlays[index];
            set_value_at_path(overlay, &write.path, write.value.clone())?;
            touched.insert(index);
        }

        for (path, target_layer) in resets {
            let index = active_layers
                .iter()
                .position(|layer| layer == target_layer)
                .ok_or_else(|| ParameterError::LayerNotActive {
                    layer: target_layer.clone(),
                })?;
            let overlay = &mut layer_overlays[index];
            if remove_value_at_path(overlay, path)? {
                touched.insert(index);
            }
        }

        let candidate = snapshot_from_parts::<T>(
            &self.state.node_fqn,
            &self.state.parameter_key,
            &self.state.layers,
            &self.state.clock,
            current.revision + 1,
            layer_overlays,
        )?;
        self.run_hooks(candidate.typed.as_ref())?;

        let persisted_layers = touched
            .iter()
            .map(|index| {
                let path =
                    self.state.layers[*index].join(format!("{}.json5", self.state.parameter_key));
                let value = candidate.layer_overlays[*index].clone();
                (path, value)
            })
            .collect::<Vec<_>>();
        write_pretty_json_batch(&persisted_layers)?;

        let diff = recursive_diff(&current.effective, &candidate.effective);
        let changed_paths = diff.iter().map(|entry| entry.path.clone()).collect();
        let snapshot = Arc::new(candidate);
        self.state.current.store(snapshot.clone());
        let _ = self.state.tx.send(snapshot.clone());
        let event = self.build_event(&current, &snapshot, diff, source);
        Ok((
            CommitOutcome {
                committed_revision: snapshot.revision,
                changed_paths,
            },
            event,
        ))
    }

    fn build_event(
        &self,
        previous: &Arc<NodeParametersSnapshot<T>>,
        current: &Arc<NodeParametersSnapshot<T>>,
        diff: Vec<RecursiveDiffEntry>,
        source: NodeParameterChangeSource,
    ) -> NodeParameterEvent {
        let changes = diff
            .into_iter()
            .map(|entry| NodeParameterChange {
                effective_source_layer: provenance_for_path(&current.provenance, &entry.path)
                    .unwrap_or_default(),
                path: entry.path,
                old_value_json: serde_json::to_string(&entry.old_value)
                    .unwrap_or_else(|_| "null".to_string()),
                new_value_json: serde_json::to_string(&entry.new_value)
                    .unwrap_or_else(|_| "null".to_string()),
            })
            .collect::<Vec<_>>();

        NodeParameterEvent {
            node_fqn: self.state.node_fqn.clone(),
            parameter_key: self.state.parameter_key.clone(),
            previous_revision: previous.revision,
            revision: current.revision,
            source,
            changed_paths: changes.iter().map(|change| change.path.clone()).collect(),
            changes,
        }
    }

    async fn publish_event(&self, event: &NodeParameterEvent) -> Result<()> {
        self.event_publisher
            .publish(event)
            .await
            .map_err(|source| ParameterError::operation("publishing parameter event", source))?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommitOutcome {
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

impl<T> std::fmt::Debug for NodeParameters<T>
where
    T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeParameters")
            .field("node_fqn", &self.inner.node_fqn)
            .field("parameter_key", &self.inner.parameter_key)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod actor_tests {
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use serde::{Deserialize, Serialize};

    use crate::context::ContextBuilder;

    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

    #[derive(Debug, Clone, Serialize, Deserialize, crate::Message)]
    #[message(name = "test_parameters::ActorLifecycleParameters")]
    struct ActorLifecycleParameters {
        enabled: bool,
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn local_mutation_reports_error_after_driver_task_aborts() {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "ros_z_parameter_actor_shutdown_{}_{}",
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("base");
        fs::create_dir_all(&base).expect("create parameter layer");
        fs::write(base.join("actor_lifecycle.json5"), r#"{ enabled: true }"#)
            .expect("write parameter file");

        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_parameter_layers([base])
            .build()
            .await
            .expect("build context");
        let node = context
            .create_node("actor_lifecycle")
            .build()
            .await
            .expect("build node");
        let parameters = node
            .bind_parameter_as::<ActorLifecycleParameters>("actor_lifecycle")
            .await
            .expect("bind parameters");

        parameters.inner.driver_task.abort();
        let err = tokio::time::timeout(Duration::from_secs(1), parameters.reload())
            .await
            .expect("reload should finish after driver abort")
            .expect_err("reload should fail after driver abort");
        assert!(err.to_string().contains("parameter actor is unavailable"));
    }
}
