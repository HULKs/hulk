use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use arc_swap::ArcSwap;
use parking_lot::Mutex;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use tokio::sync::watch;

use crate::{Message, entity::SchemaHash, node::Node};

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
        RemoteParameterServices,
        types::{NodeParameterChange, NodeParameterChangeSource, NodeParameterEvent},
    },
};

pub type ValidateHook<T> = Arc<dyn Fn(&T) -> std::result::Result<(), String> + Send + Sync>;

#[derive(Debug, Clone)]
pub struct ParameterJsonWrite {
    pub path: FieldPath,
    pub value: Value,
    pub target_layer: LayerPath,
}

#[derive(Clone)]
pub struct NodeParameters<T> {
    pub(crate) inner: Arc<NodeParametersInner<T>>,
}

pub struct NodeParametersInner<T> {
    pub(crate) node_fqn: String,
    pub(crate) parameter_key: ParameterKey,
    pub(crate) type_name: String,
    pub(crate) schema_hash: SchemaHash,
    pub(crate) layers: Vec<PathBuf>,
    clock: crate::time::Clock,
    commit_lock: Mutex<()>,
    hooks: Mutex<Vec<ValidateHook<T>>>,
    current: ArcSwap<NodeParametersSnapshot<T>>,
    tx: watch::Sender<Arc<NodeParametersSnapshot<T>>>,
    binding_state: Arc<parking_lot::Mutex<bool>>,
    remote: OnceLock<RemoteParameterServices<T>>,
}

impl<T> Drop for NodeParametersInner<T> {
    fn drop(&mut self) {
        *self.binding_state.lock() = false;
    }
}

pub trait NodeParametersExt {
    fn bind_parameter_as<T>(
        &self,
        parameter_key: impl Into<ParameterKey>,
    ) -> Result<NodeParameters<T>>
    where
        T: Serialize + DeserializeOwned + Message + Send + Sync + 'static;
}

fn block_on_parameter_future<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            tokio::runtime::RuntimeFlavor::CurrentThread => Err(ParameterError::RemoteError {
                message: "blocking parameter APIs cannot run on Tokio current_thread runtimes; use async parameter APIs from async contexts".to_string(),
            }),
            _ => Err(ParameterError::RemoteError {
                message: "blocking parameter APIs require a supported Tokio runtime".to_string(),
            }),
        }
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| ParameterError::RemoteError {
                message: format!(
                    "failed to create Tokio runtime for blocking parameter call: {err}"
                ),
            })?
            .block_on(future)
    }
}

impl NodeParametersExt for Node {
    fn bind_parameter_as<T>(
        &self,
        parameter_key: impl Into<ParameterKey>,
    ) -> Result<NodeParameters<T>>
    where
        T: Serialize + DeserializeOwned + Message + Send + Sync + 'static,
    {
        let parameter_key = parameter_key.into();
        validate_parameter_key(&parameter_key)?;
        let mut bound = self.parameter_binding_state().lock();
        if *bound {
            return Err(ParameterError::AlreadyBound {
                node_fqn: node_fqn(self),
            });
        }
        *bound = true;

        match block_on_parameter_future(bind_parameter_inner(self, parameter_key)) {
            Ok(parameters) => Ok(parameters),
            Err(err) => {
                *bound = false;
                Err(err)
            }
        }
    }
}

async fn bind_parameter_inner<T>(
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

    let schema = T::schema();
    let type_name = T::type_name().to_string();
    let schema_hash = T::schema_hash();
    node.register_schema_with_service(T::type_name(), schema)
        .map_err(|err| ParameterError::RemoteError {
            message: err.to_string(),
        })?;

    let node_fqn = node_fqn(node);
    let snapshot = load_snapshot::<T>(&node_fqn, &parameter_key, &layers, node.clock(), 0)?;
    let snapshot = Arc::new(snapshot);
    let (tx, _rx) = watch::channel(snapshot.clone());

    let binding_state = self_binding_state(node);
    let current = ArcSwap::from(snapshot);
    let inner = Arc::new(NodeParametersInner {
        node_fqn: node_fqn.clone(),
        parameter_key,
        type_name,
        schema_hash,
        layers,
        clock: node.clock().clone(),
        commit_lock: Mutex::new(()),
        hooks: Mutex::new(Vec::new()),
        current,
        tx,
        binding_state,
        remote: OnceLock::new(),
    });

    let remote = RemoteParameterServices::register(node, inner.clone()).await?;
    let _ = inner.remote.set(remote);

    Ok(NodeParameters { inner })
}

fn self_binding_state(node: &Node) -> Arc<parking_lot::Mutex<bool>> {
    node.parameter_binding_state().clone()
}

fn node_fqn(node: &Node) -> String {
    if node.namespace().is_empty() || node.namespace() == "/" {
        format!("/{}", node.name())
    } else {
        format!(
            "/{}/{}",
            node.namespace().trim_start_matches('/'),
            node.name()
        )
    }
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
    clock: &crate::time::Clock,
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
    clock: &crate::time::Clock,
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
    let typed: T = serde_json::from_value(merged.effective.clone()).map_err(|err| {
        ParameterError::DeserializationError {
            message: err.to_string(),
        }
    })?;

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
    pub fn snapshot(&self) -> Arc<NodeParametersSnapshot<T>> {
        self.inner.current.load_full()
    }

    pub fn get_json(&self, path: &str) -> Result<Value> {
        get_from_value(&self.snapshot().effective, path)?.ok_or_else(|| ParameterError::PathError {
            path: path.to_string(),
            reason: "path not found".to_string(),
        })
    }

    pub fn set_json(
        &self,
        path: &str,
        value: Value,
        target_layer: impl Into<LayerPath>,
    ) -> Result<()> {
        self.commit(
            &[ParameterJsonWrite {
                path: path.to_string(),
                value,
                target_layer: target_layer.into(),
            }],
            &[],
            None,
            NodeParameterChangeSource::LocalWrite,
        )
        .map(|_| ())
    }

    pub fn set_json_atomically(
        &self,
        changes: Vec<ParameterJsonWrite>,
        expected_revision: Option<u64>,
    ) -> Result<CommitOutcome> {
        self.commit(
            &changes,
            &[],
            expected_revision,
            NodeParameterChangeSource::LocalWrite,
        )
    }

    pub fn reset(&self, path: &str, target_layer: impl Into<LayerPath>) -> Result<()> {
        self.commit(
            &[],
            &[(path.to_string(), target_layer.into())],
            None,
            NodeParameterChangeSource::LocalWrite,
        )
        .map(|_| ())
    }

    pub fn reload(&self) -> Result<()> {
        self.reload_with_source(NodeParameterChangeSource::Reload)
            .map(|_| ())
    }

    pub(crate) fn reload_with_source(
        &self,
        source: NodeParameterChangeSource,
    ) -> Result<CommitOutcome> {
        let (outcome, event) = {
            let _commit_guard = self.inner.commit_lock.lock();
            let current = self.snapshot();

            let candidate = load_snapshot::<T>(
                &self.inner.node_fqn,
                &self.inner.parameter_key,
                &self.inner.layers,
                &self.inner.clock,
                current.revision + 1,
            )?;
            self.run_hooks(candidate.typed.as_ref())?;
            let diff = recursive_diff(&current.effective, &candidate.effective);
            let changed_paths = diff.iter().map(|entry| entry.path.clone()).collect();
            let snapshot = Arc::new(candidate);
            self.inner.current.store(snapshot.clone());
            let _ = self.inner.tx.send(snapshot.clone());
            let event = self.build_event(&current, &snapshot, diff, source);
            (
                CommitOutcome {
                    committed_revision: snapshot.revision,
                    changed_paths,
                },
                event,
            )
        };
        if let Err(err) = self.publish_event(&event) {
            tracing::warn!("[PARAM] Failed to publish parameter event: {err}");
        }
        Ok(outcome)
    }

    pub fn subscribe(&self) -> ParameterSubscription<T> {
        self.inner.tx.subscribe()
    }

    pub fn add_validation_hook<F>(&self, hook: F) -> Result<()>
    where
        F: Fn(&T) -> std::result::Result<(), String> + Send + Sync + 'static,
    {
        let hook: ValidateHook<T> = Arc::new(hook);
        let _commit_guard = self.inner.commit_lock.lock();
        self.run_hook(self.snapshot().typed.as_ref(), &hook)?;
        self.inner.hooks.lock().push(hook);
        Ok(())
    }

    fn run_hooks(&self, candidate: &T) -> Result<()> {
        for hook in self.inner.hooks.lock().iter() {
            self.run_hook(candidate, hook)?;
        }
        Ok(())
    }

    fn run_hook(&self, candidate: &T, hook: &ValidateHook<T>) -> Result<()> {
        hook(candidate).map_err(|message| ParameterError::ValidationError { message })
    }

    pub(crate) fn commit(
        &self,
        writes: &[ParameterJsonWrite],
        resets: &[(FieldPath, LayerPath)],
        expected_revision: Option<u64>,
        source: NodeParameterChangeSource,
    ) -> Result<CommitOutcome> {
        let (outcome, event) = {
            let _commit_guard = self.inner.commit_lock.lock();
            let current = self.snapshot();
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
                &self.inner.node_fqn,
                &self.inner.parameter_key,
                &self.inner.layers,
                &self.inner.clock,
                current.revision + 1,
                layer_overlays,
            )?;
            self.run_hooks(candidate.typed.as_ref())?;

            let persisted_layers = touched
                .iter()
                .map(|index| {
                    let path = self.inner.layers[*index]
                        .join(format!("{}.json5", self.inner.parameter_key));
                    let value = candidate.layer_overlays[*index].clone();
                    (path, value)
                })
                .collect::<Vec<_>>();
            write_pretty_json_batch(&persisted_layers)?;

            let diff = recursive_diff(&current.effective, &candidate.effective);
            let changed_paths = diff.iter().map(|entry| entry.path.clone()).collect();
            let snapshot = Arc::new(candidate);
            self.inner.current.store(snapshot.clone());
            let _ = self.inner.tx.send(snapshot.clone());
            let event = self.build_event(&current, &snapshot, diff, source);
            (
                CommitOutcome {
                    committed_revision: snapshot.revision,
                    changed_paths,
                },
                event,
            )
        };
        if let Err(err) = self.publish_event(&event) {
            tracing::warn!("[PARAM] Failed to publish parameter event: {err}");
        }
        Ok(outcome)
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
            node_fqn: self.inner.node_fqn.clone(),
            parameter_key: self.inner.parameter_key.clone(),
            previous_revision: previous.revision,
            revision: current.revision,
            source,
            changed_paths: changes.iter().map(|change| change.path.clone()).collect(),
            changes,
        }
    }

    fn publish_event(&self, event: &NodeParameterEvent) -> Result<()> {
        if let Some(remote) = self.inner.remote.get() {
            block_on_parameter_future(remote.publish_event(event))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommitOutcome {
    pub committed_revision: u64,
    pub changed_paths: Vec<String>,
}

impl<T> std::fmt::Debug for NodeParameters<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeParameters")
            .field("node_fqn", &self.inner.node_fqn)
            .field("parameter_key", &self.inner.parameter_key)
            .finish_non_exhaustive()
    }
}
