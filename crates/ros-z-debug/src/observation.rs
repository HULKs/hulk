use std::{
    collections::BTreeSet,
    future::Future,
    marker::PhantomData,
    sync::{Arc, Weak},
    time::Duration,
};

use parking_lot::Mutex;
use ros_z::{
    Message,
    dynamic::{DynamicPayload, TopicSchemaFingerprint, topic_schema_fingerprints_from_publishers},
    entity::{EndpointEntity, TypeInfo},
    node::Node,
    time::Time,
    topic_name::qualify_service_name,
};
use tokio::sync::{broadcast, watch};

use crate::{
    CachedSubscription, CachedSubscriptionBuilder, CachedSubscriptionStatusSnapshot,
    CachedSubscriptionUpdate, CachedSubscriptionUpdateReceiver, Error, JsonRenderPolicy, Result,
    RetentionPolicy, TargetIdentity, TopicReference,
    sample::{dynamic_record_json_value, dynamic_record_to_json_sample},
};

const UPDATE_BUFFER_CAPACITY: usize = 256;

/// Lifecycle state for a topic observation.
///
/// Observations rebuild their underlying cached subscription when the requested
/// topic, target identity, retention policy, explicit reconnect revision, or
/// relevant graph state changes. Status snapshots retain frozen previous cache
/// metadata while rebuilding, retrying, or blocked so callers can keep displaying
/// the last readable data without keeping the old subscription live.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TopicObservationStatus {
    /// The observation is resolving or building its first cache.
    Building,
    /// The observation has an active cache.
    Observing {
        /// Current status of the active cache.
        cache: CachedSubscriptionStatusSnapshot,
    },
    /// The observation is replacing an active cache after a retarget or reconnect.
    Rebuilding {
        /// Last active cache status before the rebuild started.
        previous_cache: CachedSubscriptionStatusSnapshot,
    },
    /// The observation failed to build and is waiting for a retry signal.
    ///
    /// The previous cache, when present, is frozen and no longer receives data.
    Retrying {
        /// Last active cache status, if the observation had one before retrying.
        previous_cache: Option<CachedSubscriptionStatusSnapshot>,
        /// Diagnostic message from the failed build.
        error: String,
    },
    /// The request cannot currently be resolved without additional target identity.
    ///
    /// The previous cache, when present, is frozen and no longer receives data.
    Blocked {
        /// Last active cache status, if the observation had one before blocking.
        previous_cache: Option<CachedSubscriptionStatusSnapshot>,
        /// Reason the observation cannot currently proceed.
        reason: TopicObservationBlockReason,
    },
    /// The observation is closed and will not emit future updates.
    Closed,
}

/// Reason a topic observation is blocked before it can build a cache.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TopicObservationBlockReason {
    /// A private topic reference such as `~trace` needs a target node name.
    MissingTargetNodeName { topic: String },
}

/// Live notification emitted by a topic observation.
///
/// Updates are only delivered to receivers that were already subscribed when the
/// event happened. Terminal closure is represented by
/// [`TopicObservationUpdateClosed`], not by an update variant.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TopicObservationUpdate {
    /// The observation's retained latest or windowed data changed.
    DataChanged,
    /// The observation status changed.
    StatusChanged(TopicObservationStatus),
    /// This receiver fell behind and missed updates.
    Lagged { dropped: u64 },
}

/// Error returned when a topic observation's live update stream has ended.
///
/// After this error, no future updates can arrive on that receiver. Use the
/// observation handle's retained state methods to inspect final status and data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("topic observation update stream closed")]
pub struct TopicObservationUpdateClosed;

/// Receiver for future live updates from a topic observation.
///
/// Create receivers with `TopicObservation::subscribe_updates` or
/// `DynamicTopicObservation::subscribe_updates`.
pub struct TopicObservationUpdateReceiver {
    receiver: broadcast::Receiver<TopicObservationUpdate>,
}

impl TopicObservationUpdateReceiver {
    pub(crate) fn new(receiver: broadcast::Receiver<TopicObservationUpdate>) -> Self {
        Self { receiver }
    }

    /// Wait for the next live observation update.
    ///
    /// Receivers observe updates sent after they subscribed; old updates are not
    /// replayed. `Err(TopicObservationUpdateClosed)` means the observation update
    /// stream ended and no future updates can arrive. Use the owning handle's
    /// `status()`, `latest()`, or `window()` methods to inspect retained state.
    pub async fn recv(
        &mut self,
    ) -> std::result::Result<TopicObservationUpdate, TopicObservationUpdateClosed> {
        match self.receiver.recv().await {
            Ok(update) => Ok(update),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Ok(lagged_update(dropped)),
            Err(broadcast::error::RecvError::Closed) => Err(TopicObservationUpdateClosed),
        }
    }

    /// Return the next live update if one is immediately available.
    ///
    /// Receivers observe updates sent after they subscribed; old updates are not
    /// replayed. `Ok(None)` means no update is currently queued.
    /// `Err(TopicObservationUpdateClosed)` means the live update stream ended and
    /// no future updates can arrive.
    pub fn try_recv(
        &mut self,
    ) -> std::result::Result<Option<TopicObservationUpdate>, TopicObservationUpdateClosed> {
        match self.receiver.try_recv() {
            Ok(update) => Ok(Some(update)),
            Err(broadcast::error::TryRecvError::Lagged(dropped)) => {
                Ok(Some(lagged_update(dropped)))
            }
            Err(broadcast::error::TryRecvError::Empty) => Ok(None),
            Err(broadcast::error::TryRecvError::Closed) => Err(TopicObservationUpdateClosed),
        }
    }
}

fn lagged_update(dropped: u64) -> TopicObservationUpdate {
    TopicObservationUpdate::Lagged { dropped }
}

/// Configuration shared by observations spawned from a [`TopicObserver`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TopicObserverOptions {
    target_identity: TargetIdentity,
    schema_discovery_timeout: Duration,
    retry_delay: Duration,
}

impl Default for TopicObserverOptions {
    fn default() -> Self {
        Self {
            target_identity: TargetIdentity::new("/").expect("root namespace is valid"),
            schema_discovery_timeout: Duration::from_secs(5),
            retry_delay: Duration::from_secs(1),
        }
    }
}

impl TopicObserverOptions {
    /// Create options that resolve relative topics against `namespace`.
    pub fn with_namespace(namespace: impl Into<String>) -> Result<Self> {
        let mut options = Self::default();
        options.set_namespace(namespace)?;
        Ok(options)
    }

    /// Return the default target identity used by spawned observations.
    pub fn target_identity(&self) -> &TargetIdentity {
        &self.target_identity
    }

    /// Set the default namespace used to resolve relative topic references.
    pub fn set_namespace(&mut self, namespace: impl Into<String>) -> Result<&mut Self> {
        self.target_identity.set_namespace(namespace)?;
        Ok(self)
    }

    /// Set the default node name used to resolve private topic references.
    pub fn set_node_name(&mut self, node_name: impl Into<String>) -> Result<&mut Self> {
        self.target_identity.set_node_name(node_name)?;
        Ok(self)
    }

    /// Clear the default node name used to resolve private topic references.
    pub fn clear_node_name(&mut self) -> &mut Self {
        self.target_identity.clear_node_name();
        self
    }

    /// Set the delay used after failed cache builds before retrying.
    pub fn set_retry_delay(&mut self, retry_delay: Duration) -> &mut Self {
        self.retry_delay = retry_delay;
        self
    }

    /// Set how long dynamic observations wait while discovering schema metadata.
    pub fn set_schema_discovery_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.schema_discovery_timeout = timeout;
        self
    }
}

/// Factory for topic observations that can retarget inherited observation state.
///
/// Dropping a `TopicObserver` handle does not close observations that were already
/// spawned from it. Drop the observation handles to stop their background tasks.
#[derive(Clone)]
pub struct TopicObserver {
    inner: Arc<TopicObserverInner>,
}

struct TopicObserverInner {
    node: Arc<Node>,
    options: Mutex<TopicObserverOptions>,
    target_sender: watch::Sender<TargetIdentity>,
}

struct ObservationTaskContext {
    _observer: Arc<TopicObserverInner>,
    node: Arc<Node>,
    schema_discovery_timeout: Duration,
    retry_delay: Duration,
    target_receiver: watch::Receiver<TargetIdentity>,
    graph_revisions: ros_z::graph::GraphRevisionWatch,
}

impl TopicObserver {
    /// Create an observer using `node` for underlying debug subscriptions.
    pub fn new(node: Arc<Node>, options: TopicObserverOptions) -> Self {
        let (target_sender, _) = watch::channel(options.target_identity().clone());
        Self {
            inner: Arc::new(TopicObserverInner {
                node,
                options: Mutex::new(options),
                target_sender,
            }),
        }
    }

    /// Change the namespace inherited by observations that have not overridden it.
    pub fn set_namespace(&self, namespace: impl Into<String>) -> Result<()> {
        let mut options = self.inner.options.lock();
        options.set_namespace(namespace)?;
        self.inner
            .target_sender
            .send_replace(options.target_identity().clone());
        Ok(())
    }

    /// Change the node name inherited by observations that have not overridden it.
    pub fn set_node_name(&self, node_name: impl Into<String>) -> Result<()> {
        let mut options = self.inner.options.lock();
        options.set_node_name(node_name)?;
        self.inner
            .target_sender
            .send_replace(options.target_identity().clone());
        Ok(())
    }

    /// Clear the node name inherited by observations that have not overridden it.
    pub fn clear_node_name(&self) {
        let mut options = self.inner.options.lock();
        options.clear_node_name();
        self.inner
            .target_sender
            .send_replace(options.target_identity().clone());
    }

    /// Start building a typed topic observation.
    pub fn observe_typed<T>(&self, topic: impl Into<String>) -> Result<TopicObservationBuilder<T>> {
        Ok(TopicObservationBuilder::new(
            self.clone(),
            TopicReference::new(topic.into())?,
        ))
    }

    /// Start building a dynamic topic observation that can render retained data as JSON.
    pub fn observe_dynamic(
        &self,
        topic: impl Into<String>,
    ) -> Result<DynamicTopicObservationBuilder> {
        Ok(DynamicTopicObservationBuilder::new(
            self.clone(),
            TopicReference::new(topic.into())?,
        ))
    }

    fn task_context(&self) -> ObservationTaskContext {
        let options = self.inner.options.lock().clone();
        ObservationTaskContext {
            _observer: Arc::clone(&self.inner),
            node: Arc::clone(&self.inner.node),
            schema_discovery_timeout: options.schema_discovery_timeout,
            retry_delay: options.retry_delay,
            target_receiver: self.inner.target_sender.subscribe(),
            graph_revisions: self.inner.node.graph().watch_revisions(),
        }
    }
}

/// Builder for a typed topic observation.
pub struct TopicObservationBuilder<T> {
    observer: TopicObserver,
    topic: TopicReference,
    namespace: Option<String>,
    node_name: DesiredNodeName,
    retention: RetentionPolicy,
    _value: PhantomData<T>,
}

impl<T> TopicObservationBuilder<T> {
    fn new(observer: TopicObserver, topic: TopicReference) -> Self {
        Self {
            observer,
            topic,
            namespace: None,
            node_name: DesiredNodeName::Inherit,
            retention: RetentionPolicy::LatestOnly,
            _value: PhantomData,
        }
    }

    /// Override the namespace used to resolve this observation's relative topic before spawning.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Result<Self> {
        self.namespace = Some(
            TargetIdentity::new(namespace.into())?
                .namespace()
                .to_string(),
        );
        Ok(self)
    }

    /// Return this observation to inheriting namespace changes from its observer before spawning.
    pub fn inherit_namespace(mut self) -> Self {
        self.namespace = None;
        self
    }

    /// Override the node name used to resolve this observation's private topic before spawning.
    pub fn node_name(mut self, node_name: impl Into<String>) -> Result<Self> {
        let node_name = node_name.into();
        TargetIdentity::new("/")?.with_node_name(node_name.clone())?;
        self.node_name = DesiredNodeName::Name(node_name);
        Ok(self)
    }

    /// Return this observation to inheriting node-name changes from its observer before spawning.
    pub fn inherit_node_name(mut self) -> Self {
        self.node_name = DesiredNodeName::Inherit;
        self
    }

    /// Clear this observation's node name, ignoring observer-level defaults before spawning.
    pub fn clear_node_name(mut self) -> Self {
        self.node_name = DesiredNodeName::Clear;
        self
    }

    /// Set how much data the observation retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    /// Return the topic reference requested for this observation.
    pub fn topic(&self) -> &str {
        self.topic.as_str()
    }

    /// Return the configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }
}

impl<T> TopicObservationBuilder<T>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    /// Spawn the background task and return the live observation handle.
    pub fn spawn(self) -> TopicObservation<T> {
        let (desired_sender, desired_receiver) = watch::channel(DesiredObservation {
            topic: self.topic,
            namespace: self.namespace,
            node_name: self.node_name,
            retention: self.retention,
            reconnect_revision: 0,
        });
        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);
        let state = Arc::new(Mutex::new(TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            updates: Some(updates),
        }));
        let task = self.observer.task_context();
        tokio::spawn(run_typed_observation::<T>(
            task,
            desired_receiver,
            Arc::downgrade(&state),
        ));
        TopicObservation {
            state,
            controls: TopicObservationControls { desired_sender },
        }
    }
}

impl TopicObservationBuilder<DynamicPayload> {
    fn spawn_dynamic(self) -> TopicObservation<DynamicPayload> {
        let (desired_sender, desired_receiver) = watch::channel(DesiredObservation {
            topic: self.topic,
            namespace: self.namespace,
            node_name: self.node_name,
            retention: self.retention,
            reconnect_revision: 0,
        });
        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);
        let state = Arc::new(Mutex::new(TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            updates: Some(updates),
        }));
        let task = self.observer.task_context();
        tokio::spawn(run_dynamic_observation(
            task,
            desired_receiver,
            Arc::downgrade(&state),
        ));
        TopicObservation {
            state,
            controls: TopicObservationControls { desired_sender },
        }
    }
}

/// Builder for a dynamic topic observation.
pub struct DynamicTopicObservationBuilder {
    inner: TopicObservationBuilder<DynamicPayload>,
    json_render_policy: JsonRenderPolicy,
}

impl DynamicTopicObservationBuilder {
    fn new(observer: TopicObserver, topic: TopicReference) -> Self {
        Self {
            inner: TopicObservationBuilder::new(observer, topic),
            json_render_policy: JsonRenderPolicy::default(),
        }
    }

    /// Set how much data the observation retains.
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.inner = self.inner.retention(retention);
        self
    }

    /// Override the namespace used to resolve this observation's relative topic before spawning.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Result<Self> {
        self.inner = self.inner.namespace(namespace)?;
        Ok(self)
    }

    /// Return this observation to inheriting namespace changes from its observer before spawning.
    pub fn inherit_namespace(mut self) -> Self {
        self.inner = self.inner.inherit_namespace();
        self
    }

    /// Override the node name used to resolve this observation's private topic before spawning.
    pub fn node_name(mut self, node_name: impl Into<String>) -> Result<Self> {
        self.inner = self.inner.node_name(node_name)?;
        Ok(self)
    }

    /// Return this observation to inheriting node-name changes from its observer before spawning.
    pub fn inherit_node_name(mut self) -> Self {
        self.inner = self.inner.inherit_node_name();
        self
    }

    /// Clear this observation's node name, ignoring observer-level defaults before spawning.
    pub fn clear_node_name(mut self) -> Self {
        self.inner = self.inner.clear_node_name();
        self
    }

    /// Set how retained dynamic payloads are rendered as JSON.
    pub fn json_render_policy(mut self, policy: JsonRenderPolicy) -> Self {
        self.json_render_policy = policy;
        self
    }

    /// Return the topic reference requested for this observation.
    pub fn topic(&self) -> &str {
        self.inner.topic()
    }

    /// Return the configured retention policy.
    pub fn retention_policy(&self) -> RetentionPolicy {
        self.inner.retention_policy()
    }

    /// Spawn the background task and return the live dynamic observation handle.
    pub fn spawn(self) -> DynamicTopicObservation {
        DynamicTopicObservation {
            inner: self.inner.spawn_dynamic(),
            json_render_policy: self.json_render_policy,
        }
    }
}

/// Handle for reading and retargeting an observed topic.
pub struct TopicObservation<T> {
    state: Arc<Mutex<TopicObservationState<T>>>,
    controls: TopicObservationControls,
}

impl<T> TopicObservation<T> {
    #[cfg(test)]
    fn new(desired: DesiredObservation) -> Self {
        let (desired_sender, _) = watch::channel(desired);
        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);

        Self {
            state: Arc::new(Mutex::new(TopicObservationState {
                status: TopicObservationStatus::Building,
                display_cache: None,
                updates: Some(updates),
            })),
            controls: TopicObservationControls { desired_sender },
        }
    }

    /// Change the topic reference for future rebuilds.
    pub fn set_topic(&self, topic: impl Into<String>) -> Result<()> {
        let topic = TopicReference::new(topic.into())?;
        self.controls.desired_sender.send_modify(|desired| {
            desired.topic = topic;
        });
        Ok(())
    }

    /// Override the namespace used to resolve this observation's relative topic.
    pub fn set_namespace(&self, namespace: impl Into<String>) -> Result<()> {
        let namespace = TargetIdentity::new(namespace.into())?
            .namespace()
            .to_string();
        self.controls.desired_sender.send_modify(|desired| {
            desired.namespace = Some(namespace);
        });
        Ok(())
    }

    /// Return this observation to inheriting namespace changes from its observer.
    pub fn inherit_namespace(&self) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.namespace = None;
        });
    }

    /// Override the node name used to resolve this observation's private topic.
    pub fn set_node_name(&self, node_name: impl Into<String>) -> Result<()> {
        let node_name = node_name.into();
        TargetIdentity::new("/")?.with_node_name(node_name.clone())?;
        self.controls.desired_sender.send_modify(|desired| {
            desired.node_name = DesiredNodeName::Name(node_name);
        });
        Ok(())
    }

    /// Return this observation to inheriting node-name changes from its observer.
    pub fn inherit_node_name(&self) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.node_name = DesiredNodeName::Inherit;
        });
    }

    /// Clear this observation's node name, ignoring observer-level defaults.
    pub fn clear_node_name(&self) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.node_name = DesiredNodeName::Clear;
        });
    }

    /// Change the retention policy used on the next rebuild.
    pub fn set_retention(&self, retention: RetentionPolicy) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.retention = retention;
        });
    }

    /// Force the observation to rebuild its underlying cache.
    pub fn reconnect(&self) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.reconnect_revision = desired.reconnect_revision.saturating_add(1);
        });
    }

    /// Return the current observation lifecycle status.
    pub fn status(&self) -> TopicObservationStatus {
        self.state.lock().status.clone()
    }

    /// Return the latest retained sample, if one has arrived.
    pub fn latest(&self) -> Option<Arc<crate::SampleRecord<T>>> {
        let cache = self.state.lock().display_cache.clone();
        cache?.latest()
    }

    /// Return retained samples whose source time falls inside `[start, end]`.
    ///
    /// Observations with [`RetentionPolicy::LatestOnly`] return an empty window.
    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<crate::SampleRecord<T>>> {
        let cache = self.state.lock().display_cache.clone();
        cache.map_or_else(Vec::new, |cache| cache.window(start, end))
    }

    /// Return all retained samples.
    ///
    /// Observations with [`RetentionPolicy::LatestOnly`] return an empty window.
    pub fn get_all(&self) -> Vec<Arc<crate::SampleRecord<T>>> {
        let cache = self.state.lock().display_cache.clone();
        cache.map_or_else(Vec::new, |cache| cache.get_all())
    }

    /// Return the retained sample closest to `time`, or `None` if the history is empty.
    ///
    /// Observations with [`RetentionPolicy::LatestOnly`] return an empty window.
    pub fn get_nearest(&self, time: Time) -> Option<Arc<crate::SampleRecord<T>>> {
        let cache = self.state.lock().display_cache.clone();
        cache.and_then(|cache| cache.get_nearest(time))
    }

    /// Subscribe to future status and data update notifications.
    ///
    /// The receiver is a live stream and does not replay updates that happened
    /// before subscription. When the observation closes, the update stream ends;
    /// call [`Self::status`] to inspect the terminal status and [`Self::latest`]
    /// or [`Self::window`] to inspect retained data.
    pub fn subscribe_updates(
        &self,
    ) -> std::result::Result<TopicObservationUpdateReceiver, TopicObservationUpdateClosed> {
        let state = self.state.lock();
        if matches!(state.status, TopicObservationStatus::Closed) || state.updates.is_none() {
            return Err(TopicObservationUpdateClosed);
        }
        let updates = state
            .updates
            .as_ref()
            .expect("open observations keep an update sender")
            .subscribe();
        Ok(TopicObservationUpdateReceiver::new(updates))
    }
}

/// Handle for reading and retargeting an observed dynamic topic.
pub struct DynamicTopicObservation {
    inner: TopicObservation<DynamicPayload>,
    json_render_policy: JsonRenderPolicy,
}

impl DynamicTopicObservation {
    /// Render the latest retained dynamic payload as JSON.
    pub fn latest_json(&self) -> Option<serde_json::Value> {
        self.inner
            .latest()
            .map(|record| dynamic_record_json_value(record.as_ref(), self.json_render_policy))
    }

    /// Render the latest retained dynamic payload as JSON with sample metadata.
    pub fn latest_json_record(&self) -> Option<crate::SampleRecord<serde_json::Value>> {
        self.inner
            .latest()
            .map(|record| dynamic_record_to_json_sample(record, self.json_render_policy))
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON values.
    pub fn window_json(&self, start: Time, end: Time) -> Vec<serde_json::Value> {
        self.inner
            .window(start, end)
            .iter()
            .map(|record| dynamic_record_json_value(record.as_ref(), self.json_render_policy))
            .collect()
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON records.
    pub fn window_json_records(
        &self,
        start: Time,
        end: Time,
    ) -> Vec<crate::SampleRecord<serde_json::Value>> {
        self.inner
            .window(start, end)
            .into_iter()
            .map(|record| dynamic_record_to_json_sample(record, self.json_render_policy))
            .collect()
    }

    /// Return the current observation lifecycle status.
    pub fn status(&self) -> TopicObservationStatus {
        self.inner.status()
    }

    /// Return the latest retained dynamic sample, if one has arrived.
    pub fn latest(&self) -> Option<Arc<crate::SampleRecord<DynamicPayload>>> {
        self.inner.latest()
    }

    /// Return retained dynamic samples whose source time falls inside `[start, end]`.
    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<crate::SampleRecord<DynamicPayload>>> {
        self.inner.window(start, end)
    }

    /// Change the topic reference for future rebuilds.
    pub fn set_topic(&self, topic: impl Into<String>) -> Result<()> {
        self.inner.set_topic(topic)
    }

    /// Override the namespace used to resolve this observation's relative topic.
    pub fn set_namespace(&self, namespace: impl Into<String>) -> Result<()> {
        self.inner.set_namespace(namespace)
    }

    /// Return this observation to inheriting namespace changes from its observer.
    pub fn inherit_namespace(&self) {
        self.inner.inherit_namespace();
    }

    /// Override the node name used to resolve this observation's private topic.
    pub fn set_node_name(&self, node_name: impl Into<String>) -> Result<()> {
        self.inner.set_node_name(node_name)
    }

    /// Return this observation to inheriting node-name changes from its observer.
    pub fn inherit_node_name(&self) {
        self.inner.inherit_node_name();
    }

    /// Clear this observation's node name, ignoring observer-level defaults.
    pub fn clear_node_name(&self) {
        self.inner.clear_node_name();
    }

    /// Change the retention policy used on the next rebuild.
    pub fn set_retention(&self, retention: RetentionPolicy) {
        self.inner.set_retention(retention);
    }

    /// Force the observation to rebuild its underlying cache.
    pub fn reconnect(&self) {
        self.inner.reconnect();
    }

    /// Subscribe to future status and data update notifications.
    pub fn subscribe_updates(
        &self,
    ) -> std::result::Result<TopicObservationUpdateReceiver, TopicObservationUpdateClosed> {
        self.inner.subscribe_updates()
    }
}

fn format_error_chain(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(error) = source {
        let source_message = error.to_string();
        let already_rendered = message == source_message
            || message
                .strip_suffix(&source_message)
                .is_some_and(|prefix| prefix.ends_with(": "));
        if !already_rendered {
            message.push_str(": ");
            message.push_str(&source_message);
        }
        source = error.source();
    }
    message
}

struct TopicObservationState<T> {
    status: TopicObservationStatus,
    display_cache: Option<CachedSubscription<T>>,
    updates: Option<broadcast::Sender<TopicObservationUpdate>>,
}

struct TopicObservationControls {
    desired_sender: watch::Sender<DesiredObservation>,
}

#[derive(Clone)]
struct DesiredObservation {
    topic: TopicReference,
    namespace: Option<String>,
    node_name: DesiredNodeName,
    retention: RetentionPolicy,
    reconnect_revision: u64,
}

#[derive(Clone)]
enum DesiredNodeName {
    Inherit,
    Name(String),
    Clear,
}

impl DesiredObservation {
    #[cfg(test)]
    fn new(topic: TopicReference, retention: RetentionPolicy) -> Self {
        Self {
            topic,
            namespace: None,
            node_name: DesiredNodeName::Inherit,
            retention,
            reconnect_revision: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TopicGraphFingerprint {
    publishers: Vec<TopicSchemaFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SchemaServiceFingerprint {
    service_name: String,
    node_namespace: String,
    node_name: String,
    endpoint_id: usize,
    type_info: TypeInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DynamicGraphFingerprint {
    topic: TopicGraphFingerprint,
    schema_services: Vec<SchemaServiceFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObservationBuildKey {
    resolved_topic: String,
    retention: RetentionPolicy,
    reconnect_revision: u64,
}

struct ResolvedObservationTarget {
    identity: TargetIdentity,
    resolved_topic: String,
    build_key: ObservationBuildKey,
    graph_fingerprint: TopicGraphFingerprint,
}

enum GraphChangeFilter<'a> {
    Topic {
        node: &'a Node,
        resolved_topic: &'a str,
        fingerprint: TopicGraphFingerprint,
    },
    Dynamic {
        node: &'a Node,
        resolved_topic: &'a str,
        fingerprint: DynamicGraphFingerprint,
    },
}

fn topic_graph_fingerprint_from_publishers(publishers: &[EndpointEntity]) -> TopicGraphFingerprint {
    TopicGraphFingerprint {
        publishers: topic_schema_fingerprints_from_publishers(publishers),
    }
}

fn topic_graph_fingerprint(node: &Node, resolved_topic: &str) -> TopicGraphFingerprint {
    let graph = node.graph().lock();
    let publishers = graph
        .publishers_on(resolved_topic)
        .cloned()
        .collect::<Vec<_>>();
    topic_graph_fingerprint_from_publishers(&publishers)
}

fn schema_service_name_for_publisher(publisher: &EndpointEntity) -> Option<String> {
    qualify_service_name(
        "~get_schema",
        publisher.node.namespace.as_str(),
        publisher.node.name.as_str(),
    )
    .ok()
}

fn dynamic_graph_fingerprint(node: &Node, resolved_topic: &str) -> DynamicGraphFingerprint {
    let (publishers, mut schema_services) = {
        let graph = node.graph().lock();
        let publishers = graph
            .publishers_on(resolved_topic)
            .cloned()
            .collect::<Vec<_>>();
        let schema_service_names = publishers
            .iter()
            .filter_map(schema_service_name_for_publisher)
            .collect::<BTreeSet<_>>();
        let schema_services = if schema_service_names.is_empty() {
            Vec::new()
        } else {
            graph
                .services()
                .filter(|endpoint| schema_service_names.contains(&endpoint.topic))
                .map(|service| SchemaServiceFingerprint {
                    service_name: service.topic.clone(),
                    node_namespace: service.node.namespace.clone(),
                    node_name: service.node.name.clone(),
                    endpoint_id: service.id,
                    type_info: service.type_info.clone(),
                })
                .collect::<Vec<_>>()
        };
        (publishers, schema_services)
    };
    let topic = topic_graph_fingerprint_from_publishers(&publishers);
    schema_services.sort_by(|left, right| {
        (
            &left.service_name,
            &left.node_namespace,
            &left.node_name,
            left.endpoint_id,
            &left.type_info.name,
            &left.type_info.hash.0,
        )
            .cmp(&(
                &right.service_name,
                &right.node_namespace,
                &right.node_name,
                right.endpoint_id,
                &right.type_info.name,
                &right.type_info.hash.0,
            ))
    });
    schema_services.dedup();

    DynamicGraphFingerprint {
        topic,
        schema_services,
    }
}

fn topic_graph_fingerprint_changed(
    node: &Node,
    resolved_topic: &str,
    current: &mut TopicGraphFingerprint,
) -> bool {
    let next = topic_graph_fingerprint(node, resolved_topic);
    if next == *current {
        return false;
    }
    *current = next;
    true
}

fn topic_graph_fingerprint_changed_from(
    node: &Node,
    resolved_topic: &str,
    previous: &TopicGraphFingerprint,
) -> bool {
    topic_graph_fingerprint(node, resolved_topic) != *previous
}

fn dynamic_graph_fingerprint_changed(
    node: &Node,
    resolved_topic: &str,
    current: &mut DynamicGraphFingerprint,
) -> bool {
    let next = dynamic_graph_fingerprint(node, resolved_topic);
    if next == *current {
        return false;
    }
    *current = next;
    true
}

fn dynamic_graph_fingerprint_changed_from(
    node: &Node,
    resolved_topic: &str,
    previous: &DynamicGraphFingerprint,
) -> bool {
    dynamic_graph_fingerprint(node, resolved_topic) != *previous
}

fn preinstall_observing_graph_fingerprint(
    graph_revisions: &mut ros_z::graph::GraphRevisionWatch,
    node: &Node,
    resolved_topic: &str,
    build_fingerprint: &TopicGraphFingerprint,
) -> Option<TopicGraphFingerprint> {
    graph_revisions.mark_seen();
    let current = topic_graph_fingerprint(node, resolved_topic);
    (&current == build_fingerprint).then_some(current)
}

fn preinstall_dynamic_observing_graph_fingerprint(
    graph_revisions: &mut ros_z::graph::GraphRevisionWatch,
    node: &Node,
    resolved_topic: &str,
    build_fingerprint: &DynamicGraphFingerprint,
) -> Option<TopicGraphFingerprint> {
    graph_revisions.mark_seen();
    let current = dynamic_graph_fingerprint(node, resolved_topic);
    (&current == build_fingerprint).then_some(current.topic)
}

fn effective_identity(
    observer_identity: &TargetIdentity,
    desired: &DesiredObservation,
) -> Result<TargetIdentity> {
    let mut identity = TargetIdentity::new(
        desired
            .namespace
            .as_deref()
            .unwrap_or_else(|| observer_identity.namespace()),
    )?;
    let node_name = match &desired.node_name {
        DesiredNodeName::Inherit => observer_identity.node_name(),
        DesiredNodeName::Name(node_name) => Some(node_name.as_str()),
        DesiredNodeName::Clear => None,
    };
    if let Some(node_name) = node_name {
        identity.set_node_name(node_name)?;
    }
    Ok(identity)
}

async fn build_typed_cache<T>(
    node: Arc<Node>,
    schema_discovery_timeout: Duration,
    identity: TargetIdentity,
    desired: DesiredObservation,
) -> Result<CachedSubscription<T>>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    CachedSubscriptionBuilder::new(node, desired.topic.as_str())?
        .target_identity(identity)
        .schema_discovery_timeout(schema_discovery_timeout)
        .retention(desired.retention)
        .build_typed::<T>()
        .await
}

async fn build_dynamic_cache(
    node: Arc<Node>,
    schema_discovery_timeout: Duration,
    identity: TargetIdentity,
    desired: DesiredObservation,
) -> Result<CachedSubscription<DynamicPayload>> {
    CachedSubscriptionBuilder::new(node, desired.topic.as_str())?
        .target_identity(identity)
        .schema_discovery_timeout(schema_discovery_timeout)
        .retention(desired.retention)
        .build_dynamic()
        .await
}

enum BuildWait<T> {
    Completed(Result<CachedSubscription<T>>),
    RebuildRequested,
    Closed,
}

async fn wait_for_build<T, F>(
    build: F,
    current_key: &ObservationBuildKey,
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    graph_revisions: &mut ros_z::graph::GraphRevisionWatch,
    graph_filter: Option<GraphChangeFilter<'_>>,
) -> BuildWait<T>
where
    F: Future<Output = Result<CachedSubscription<T>>>,
{
    let mut graph_filter = graph_filter;
    tokio::pin!(build);
    loop {
        tokio::select! {
            result = &mut build => return BuildWait::Completed(result),
            result = desired_receiver.changed() => {
                match result {
                    Ok(()) if current_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        current_key,
                    ) => return BuildWait::RebuildRequested,
                    Ok(()) => {}
                    Err(_) => return BuildWait::Closed,
                }
            }
            result = target_receiver.changed() => {
                match result {
                    Ok(()) if current_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        current_key,
                    ) => return BuildWait::RebuildRequested,
                    Ok(()) => {}
                    Err(_) => return BuildWait::Closed,
                }
            }
            revision = graph_revisions.changed() => {
                if revision.is_none() {
                    return BuildWait::Closed;
                }
                if let Some(filter) = &mut graph_filter
                    && graph_change_filter_changed(filter)
                {
                    return BuildWait::RebuildRequested;
                }
            }
        }
    }
}

fn build_inputs_changed(
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    current_key: &ObservationBuildKey,
) -> std::result::Result<bool, watch::error::RecvError> {
    let desired_changed = desired_receiver.has_changed()?;
    let target_changed = target_receiver.has_changed()?;
    if !desired_changed && !target_changed {
        return Ok(false);
    }

    Ok(current_build_key_changed_after_pending_update(
        desired_receiver,
        target_receiver,
        desired_changed,
        target_changed,
        current_key,
    ))
}

fn current_build_key_changed(
    desired_receiver: &watch::Receiver<DesiredObservation>,
    target_receiver: &watch::Receiver<TargetIdentity>,
    current_key: &ObservationBuildKey,
) -> bool {
    observation_build_key(&target_receiver.borrow(), &desired_receiver.borrow())
        .is_none_or(|next_key| next_key != *current_key)
}

fn current_build_key_changed_after_pending_update(
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    desired_changed: bool,
    target_changed: bool,
    current_key: &ObservationBuildKey,
) -> bool {
    let desired = if desired_changed {
        desired_receiver.borrow_and_update().clone()
    } else {
        desired_receiver.borrow().clone()
    };
    let observer_identity = if target_changed {
        target_receiver.borrow_and_update().clone()
    } else {
        target_receiver.borrow().clone()
    };

    observation_build_key(&observer_identity, &desired)
        .is_none_or(|next_key| next_key != *current_key)
}

fn observation_build_key(
    observer_identity: &TargetIdentity,
    desired: &DesiredObservation,
) -> Option<ObservationBuildKey> {
    let identity = effective_identity(observer_identity, desired).ok()?;
    let resolved_topic = desired.topic.resolve(&identity).ok()?;
    Some(ObservationBuildKey {
        resolved_topic,
        retention: desired.retention,
        reconnect_revision: desired.reconnect_revision,
    })
}

async fn run_typed_observation<T>(
    mut task: ObservationTaskContext,
    mut desired_receiver: watch::Receiver<DesiredObservation>,
    state: Weak<Mutex<TopicObservationState<T>>>,
) where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    loop {
        task.graph_revisions.mark_seen();
        let desired = desired_receiver.borrow_and_update().clone();
        let observer_identity = task.target_receiver.borrow_and_update().clone();

        if !set_rebuild_status(&state) {
            return;
        }

        let target = match resolve_build_target(&task.node, &observer_identity, &desired) {
            Ok(target) => target,
            Err(Error::MissingTargetNodeName { .. }) => {
                if !set_blocked_status(
                    &state,
                    TopicObservationBlockReason::MissingTargetNodeName {
                        topic: desired.topic.as_str().to_string(),
                    },
                ) {
                    return;
                }
                if !wait_for_blocked_signal(&mut desired_receiver, &mut task.target_receiver).await
                {
                    close_observation(&state);
                    return;
                }
                continue;
            }
            Err(error) => {
                if !set_retrying_status(&state, &error) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    None,
                    None,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
                continue;
            }
        };

        let build = build_typed_cache::<T>(
            Arc::clone(&task.node),
            task.schema_discovery_timeout,
            target.identity,
            desired,
        );
        match wait_for_build(
            build,
            &target.build_key,
            &mut desired_receiver,
            &mut task.target_receiver,
            &mut task.graph_revisions,
            Some(GraphChangeFilter::Topic {
                node: task.node.as_ref(),
                resolved_topic: target.resolved_topic.as_str(),
                fingerprint: target.graph_fingerprint.clone(),
            }),
        )
        .await
        {
            BuildWait::Completed(Ok(built_cache)) => {
                match build_inputs_changed(
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &target.build_key,
                ) {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(_) => {
                        close_observation(&state);
                        return;
                    }
                }
                let Some(mut cache_updates) = subscribe_built_cache_updates(&built_cache) else {
                    continue;
                };
                let Some(graph_fingerprint) = preinstall_observing_graph_fingerprint(
                    &mut task.graph_revisions,
                    task.node.as_ref(),
                    target.resolved_topic.as_str(),
                    &target.graph_fingerprint,
                ) else {
                    continue;
                };
                if !install_cache(&state, built_cache) {
                    return;
                }
                if !refresh_observing_status(&state) {
                    return;
                }

                if !wait_for_observing_rebuild(
                    &state,
                    (
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        target.build_key,
                        graph_fingerprint,
                    ),
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    &mut cache_updates,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::Completed(Err(error)) => {
                if topic_graph_fingerprint_changed_from(
                    task.node.as_ref(),
                    target.resolved_topic.as_str(),
                    &target.graph_fingerprint,
                ) {
                    continue;
                }
                if !set_retrying_status(&state, &error) {
                    return;
                }
                let retry_build_key = target.build_key.clone();
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    Some(&retry_build_key),
                    Some(GraphChangeFilter::Topic {
                        node: task.node.as_ref(),
                        resolved_topic: target.resolved_topic.as_str(),
                        fingerprint: target.graph_fingerprint,
                    }),
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::RebuildRequested => continue,
            BuildWait::Closed => {
                close_observation(&state);
                return;
            }
        }
    }
}

async fn run_dynamic_observation(
    mut task: ObservationTaskContext,
    mut desired_receiver: watch::Receiver<DesiredObservation>,
    state: Weak<Mutex<TopicObservationState<DynamicPayload>>>,
) {
    loop {
        task.graph_revisions.mark_seen();
        let desired = desired_receiver.borrow_and_update().clone();
        let observer_identity = task.target_receiver.borrow_and_update().clone();

        if !set_rebuild_status(&state) {
            return;
        }

        let target = match resolve_build_target(&task.node, &observer_identity, &desired) {
            Ok(target) => target,
            Err(Error::MissingTargetNodeName { .. }) => {
                if !set_blocked_status(
                    &state,
                    TopicObservationBlockReason::MissingTargetNodeName {
                        topic: desired.topic.as_str().to_string(),
                    },
                ) {
                    return;
                }
                if !wait_for_blocked_signal(&mut desired_receiver, &mut task.target_receiver).await
                {
                    close_observation(&state);
                    return;
                }
                continue;
            }
            Err(error) => {
                if !set_retrying_status(&state, &error) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    None,
                    None,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
                continue;
            }
        };

        let dynamic_graph_fingerprint =
            dynamic_graph_fingerprint(task.node.as_ref(), target.resolved_topic.as_str());

        let build = build_dynamic_cache(
            Arc::clone(&task.node),
            task.schema_discovery_timeout,
            target.identity,
            desired,
        );
        match wait_for_build(
            build,
            &target.build_key,
            &mut desired_receiver,
            &mut task.target_receiver,
            &mut task.graph_revisions,
            Some(GraphChangeFilter::Dynamic {
                node: task.node.as_ref(),
                resolved_topic: target.resolved_topic.as_str(),
                fingerprint: dynamic_graph_fingerprint.clone(),
            }),
        )
        .await
        {
            BuildWait::Completed(Ok(built_cache)) => {
                match build_inputs_changed(
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &target.build_key,
                ) {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(_) => {
                        close_observation(&state);
                        return;
                    }
                }
                let Some(mut cache_updates) = subscribe_built_cache_updates(&built_cache) else {
                    continue;
                };
                let Some(graph_fingerprint) = preinstall_dynamic_observing_graph_fingerprint(
                    &mut task.graph_revisions,
                    task.node.as_ref(),
                    target.resolved_topic.as_str(),
                    &dynamic_graph_fingerprint,
                ) else {
                    continue;
                };
                if !install_cache(&state, built_cache) {
                    return;
                }
                if !refresh_observing_status(&state) {
                    return;
                }

                if !wait_for_observing_rebuild(
                    &state,
                    (
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        target.build_key,
                        graph_fingerprint,
                    ),
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    &mut cache_updates,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::Completed(Err(error)) => {
                if dynamic_graph_fingerprint_changed_from(
                    task.node.as_ref(),
                    target.resolved_topic.as_str(),
                    &dynamic_graph_fingerprint,
                ) {
                    continue;
                }
                if !set_retrying_status(&state, &error) {
                    return;
                }
                let retry_build_key = target.build_key.clone();
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_revisions,
                    Some(&retry_build_key),
                    Some(GraphChangeFilter::Dynamic {
                        node: task.node.as_ref(),
                        resolved_topic: target.resolved_topic.as_str(),
                        fingerprint: dynamic_graph_fingerprint,
                    }),
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::RebuildRequested => continue,
            BuildWait::Closed => {
                close_observation(&state);
                return;
            }
        }
    }
}

fn resolve_build_target(
    node: &Node,
    observer_identity: &TargetIdentity,
    desired: &DesiredObservation,
) -> Result<ResolvedObservationTarget> {
    let identity = effective_identity(observer_identity, desired)?;
    let resolved_topic = desired.topic.resolve(&identity)?;
    let build_key = ObservationBuildKey {
        resolved_topic: resolved_topic.clone(),
        retention: desired.retention,
        reconnect_revision: desired.reconnect_revision,
    };
    let graph_fingerprint = topic_graph_fingerprint(node, &resolved_topic);
    Ok(ResolvedObservationTarget {
        identity,
        resolved_topic,
        build_key,
        graph_fingerprint,
    })
}

async fn wait_for_blocked_signal(
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
) -> bool {
    tokio::select! {
        result = desired_receiver.changed() => result.is_ok(),
        result = target_receiver.changed() => result.is_ok(),
    }
}

async fn wait_for_retry_signal(
    retry_delay: Duration,
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    graph_revisions: &mut ros_z::graph::GraphRevisionWatch,
    current_key: Option<&ObservationBuildKey>,
    graph_filter: Option<GraphChangeFilter<'_>>,
) -> bool {
    let mut graph_filter = graph_filter;
    let retry_sleep = tokio::time::sleep(retry_delay);
    tokio::pin!(retry_sleep);
    loop {
        tokio::select! {
            _ = &mut retry_sleep => return true,
            result = desired_receiver.changed() => {
                match result {
                    Ok(()) if retry_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        current_key,
                    ) => return true,
                    Ok(()) => {}
                    Err(_) => return false,
                }
            }
            result = target_receiver.changed() => {
                match result {
                    Ok(()) if retry_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        current_key,
                    ) => return true,
                    Ok(()) => {}
                    Err(_) => return false,
                }
            }
            revision = graph_revisions.changed() => {
                if revision.is_none() {
                    return false;
                }
                if let Some(filter) = &mut graph_filter
                    && graph_change_filter_changed(filter)
                {
                    return true;
                }
            }
        }
    }
}

fn retry_build_key_changed(
    desired_receiver: &watch::Receiver<DesiredObservation>,
    target_receiver: &watch::Receiver<TargetIdentity>,
    current_key: Option<&ObservationBuildKey>,
) -> bool {
    current_key.is_none_or(|current_key| {
        current_build_key_changed(desired_receiver, target_receiver, current_key)
    })
}

fn graph_change_filter_changed(filter: &mut GraphChangeFilter<'_>) -> bool {
    match filter {
        GraphChangeFilter::Topic {
            node,
            resolved_topic,
            fingerprint,
        } => topic_graph_fingerprint_changed(node, resolved_topic, fingerprint),
        GraphChangeFilter::Dynamic {
            node,
            resolved_topic,
            fingerprint,
        } => dynamic_graph_fingerprint_changed(node, resolved_topic, fingerprint),
    }
}

async fn wait_for_observing_rebuild<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    graph_filter: (&Node, &str, ObservationBuildKey, TopicGraphFingerprint),
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    graph_revisions: &mut ros_z::graph::GraphRevisionWatch,
    cache_updates: &mut CachedSubscriptionUpdateReceiver,
) -> bool {
    let (node, resolved_topic, build_key, mut graph_fingerprint) = graph_filter;
    loop {
        let rebuild = tokio::select! {
            result = desired_receiver.changed() => {
                match result {
                    Ok(()) => current_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        &build_key,
                    ),
                    Err(_) => return false,
                }
            }
            result = target_receiver.changed() => {
                match result {
                    Ok(()) => current_build_key_changed(
                        desired_receiver,
                        target_receiver,
                        &build_key,
                    ),
                    Err(_) => return false,
                }
            }
            revision = graph_revisions.changed() => {
                if revision.is_none() {
                    return false;
                }
                topic_graph_fingerprint_changed(node, resolved_topic, &mut graph_fingerprint)
            }
            update = cache_updates.recv() => {
                match update {
                    Ok(CachedSubscriptionUpdate::DataChanged) => {
                        send_observation_update(state, TopicObservationUpdate::DataChanged);
                        false
                    }
                    Ok(CachedSubscriptionUpdate::StatusChanged(_)) => {
                        !refresh_observing_status(state)
                    }
                    Ok(CachedSubscriptionUpdate::Lagged { dropped }) => {
                        send_observation_update(
                            state,
                            TopicObservationUpdate::Lagged { dropped },
                        );
                        !refresh_observing_status(state)
                    }
                    Err(_) => true,
                }
            }
        };

        if rebuild {
            return true;
        }
    }
}

fn previous_cache<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
) -> Option<CachedSubscriptionStatusSnapshot> {
    state.upgrade().and_then(|state| {
        let state = state.lock();
        match &state.status {
            TopicObservationStatus::Rebuilding { previous_cache } => Some(previous_cache.clone()),
            TopicObservationStatus::Retrying { previous_cache, .. }
            | TopicObservationStatus::Blocked { previous_cache, .. } => previous_cache.clone(),
            _ => state.display_cache.as_ref().map(CachedSubscription::status),
        }
    })
}

fn freeze_display_cache<T>(state: &Weak<Mutex<TopicObservationState<T>>>) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };
    let cache = {
        let state = state.lock();
        if state.updates.is_none() {
            return false;
        }
        state.display_cache.clone()
    };
    if let Some(cache) = cache {
        cache.close_retaining_samples();
    }
    true
}

fn set_rebuild_status<T>(state: &Weak<Mutex<TopicObservationState<T>>>) -> bool {
    let previous_cache = previous_cache(state);
    if previous_cache.is_some() && !freeze_display_cache(state) {
        return false;
    }
    let status = match previous_cache {
        Some(previous_cache) => TopicObservationStatus::Rebuilding { previous_cache },
        None => TopicObservationStatus::Building,
    };
    set_observation_status(state, status)
}

fn set_retrying_status<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    error: &dyn std::error::Error,
) -> bool {
    let previous_cache = previous_cache(state);
    if !freeze_display_cache(state) {
        return false;
    }
    set_observation_status(
        state,
        TopicObservationStatus::Retrying {
            previous_cache,
            error: format_error_chain(error),
        },
    )
}

fn set_blocked_status<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    reason: TopicObservationBlockReason,
) -> bool {
    let previous_cache = previous_cache(state);
    if !freeze_display_cache(state) {
        return false;
    }
    set_observation_status(
        state,
        TopicObservationStatus::Blocked {
            previous_cache,
            reason,
        },
    )
}

fn install_cache<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    cache: CachedSubscription<T>,
) -> bool {
    let has_retained_sample = cache.latest().is_some();
    let status = TopicObservationStatus::Observing {
        cache: cache.status(),
    };
    let Some(state) = state.upgrade() else {
        return false;
    };
    let (updates, status_changed) = {
        let mut state = state.lock();
        if state.updates.is_none() {
            return false;
        }
        let status_changed = state.status != status;
        state.display_cache = Some(cache);
        state.status = status.clone();
        let updates = (status_changed || has_retained_sample)
            .then(|| state.updates.clone())
            .flatten();
        (updates, status_changed)
    };
    if let Some(updates) = updates {
        if status_changed {
            let _ = updates.send(TopicObservationUpdate::StatusChanged(status));
        }
        if has_retained_sample {
            let _ = updates.send(TopicObservationUpdate::DataChanged);
        }
    }
    true
}

fn refresh_observing_status<T>(state: &Weak<Mutex<TopicObservationState<T>>>) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };
    let status = {
        let state = state.lock();
        let Some(cache) = &state.display_cache else {
            return false;
        };
        TopicObservationStatus::Observing {
            cache: cache.status(),
        }
    };
    set_observation_status(&Arc::downgrade(&state), status)
}

fn subscribe_built_cache_updates<T>(
    cache: &CachedSubscription<T>,
) -> Option<CachedSubscriptionUpdateReceiver> {
    cache.subscribe_updates().ok()
}

fn set_observation_status<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    status: TopicObservationStatus,
) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };
    let updates = {
        let mut state = state.lock();
        if state.status == status {
            return state.updates.is_some();
        }
        state.status = status.clone();
        state.updates.clone()
    };
    let Some(updates) = updates else {
        return false;
    };
    let _ = updates.send(TopicObservationUpdate::StatusChanged(status));
    true
}

fn close_observation<T>(state: &Weak<Mutex<TopicObservationState<T>>>) {
    let Some(state) = state.upgrade() else {
        return;
    };
    let display_cache = {
        let mut state = state.lock();
        state.status = TopicObservationStatus::Closed;
        state.updates.take();
        state.display_cache.clone()
    };
    if let Some(cache) = display_cache {
        cache.close_retaining_samples();
    }
}

fn send_observation_update<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    update: TopicObservationUpdate,
) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };
    let updates = state.lock().updates.clone();
    let Some(updates) = updates else {
        return false;
    };
    let _ = updates.send(update);
    true
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use parking_lot::Mutex;
    use ros_z::{prelude::*, time::Time};
    use tokio::sync::{broadcast, oneshot, watch};

    use super::{
        DesiredNodeName, DesiredObservation, TopicObservation, TopicObservationBlockReason,
        TopicObservationStatus, TopicObservationUpdate, TopicObservationUpdateClosed,
        TopicObservationUpdateReceiver,
    };
    use crate::{
        CachedSubscriptionStatus, CachedSubscriptionStatusSnapshot, CachedSubscriptionUpdate,
        CachedSubscriptionUpdateReceiver, RetentionPolicy, SampleMetadata, SampleRecord,
        TargetIdentity, TopicReference, cache::CachedSubscriptionState,
    };

    fn test_type_info() -> ros_z::TypeInfo {
        ros_z::TypeInfo::new("test_msgs::DebugValue", ros_z::SchemaHash::zero())
    }

    fn publisher_endpoint(
        node_name: &str,
        hash: ros_z::SchemaHash,
    ) -> ros_z::entity::EndpointEntity {
        ros_z::entity::EndpointEntity {
            id: 1,
            node: ros_z::entity::NodeEntity {
                z_id: Default::default(),
                id: 2,
                name: node_name.to_string(),
                namespace: "/".to_string(),
            },
            kind: ros_z::entity::EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: ros_z::TypeInfo::new("std_msgs::String", hash),
            qos: Default::default(),
        }
    }

    fn test_publication_id() -> ros_z::pubsub::PublicationId {
        ros_z::pubsub::Received {
            message: (),
            transport_time: None,
            source_time: Time::zero(),
            sequence_number: 1,
            source_global_id: ros_z::EndpointGlobalId::from([7; 16]),
        }
        .publication_id()
    }

    fn string_sample_record(value: &str) -> Arc<SampleRecord<String>> {
        Arc::new(SampleRecord {
            value: value.to_string(),
            source_time: Time::zero(),
            transport_time: None,
            publication_id: test_publication_id(),
            metadata: Arc::new(SampleMetadata {
                topic_reference: TopicReference::new("status").unwrap(),
                resolved_topic: "/status".to_string(),
                type_info: test_type_info(),
            }),
        })
    }

    #[test]
    fn building_status_has_no_cache() {
        let status = TopicObservationStatus::Building;

        assert!(matches!(status, TopicObservationStatus::Building));
    }

    #[test]
    fn missing_target_node_block_reason_carries_topic() {
        let reason = TopicObservationBlockReason::MissingTargetNodeName {
            topic: "~trace".to_string(),
        };

        assert!(matches!(
            reason,
            TopicObservationBlockReason::MissingTargetNodeName { topic } if topic == "~trace"
        ));
    }

    #[test]
    fn observation_inherits_observer_target_identity_by_default() {
        let observer_identity = TargetIdentity::new("/42")
            .unwrap()
            .with_node_name("behavior")
            .unwrap();
        let desired = DesiredObservation {
            topic: TopicReference::new("~trace").unwrap(),
            namespace: None,
            node_name: DesiredNodeName::Inherit,
            retention: RetentionPolicy::LatestOnly,
            reconnect_revision: 0,
        };

        let effective = super::effective_identity(&observer_identity, &desired).unwrap();

        assert_eq!(effective.namespace(), "/42");
        assert_eq!(effective.node_name(), Some("behavior"));
    }

    #[test]
    fn observation_target_identity_overrides_observer_defaults() {
        let observer_identity = TargetIdentity::new("/42")
            .unwrap()
            .with_node_name("behavior")
            .unwrap();
        let desired = DesiredObservation {
            topic: TopicReference::new("~trace").unwrap(),
            namespace: Some("/99".to_string()),
            node_name: DesiredNodeName::Name("vision".to_string()),
            retention: RetentionPolicy::LatestOnly,
            reconnect_revision: 0,
        };

        let effective = super::effective_identity(&observer_identity, &desired).unwrap();

        assert_eq!(effective.namespace(), "/99");
        assert_eq!(effective.node_name(), Some("vision"));
    }

    #[test]
    fn topic_observer_options_clear_node_name_removes_default_private_target() {
        let mut options = super::TopicObserverOptions::with_namespace("/42").unwrap();
        options.set_node_name("behavior_node").unwrap();

        options.clear_node_name();

        assert_eq!(options.target_identity().node_name(), None);
    }

    #[test]
    fn schema_service_name_for_publisher_rejects_invalid_node_identity() {
        let publisher = ros_z::entity::EndpointEntity {
            id: 1,
            node: ros_z::entity::NodeEntity {
                z_id: Default::default(),
                id: 2,
                name: "bad%node".to_string(),
                namespace: "/".to_string(),
            },
            kind: ros_z::entity::EndpointKind::Publisher,
            topic: "/chatter".to_string(),
            type_info: test_type_info(),
            qos: Default::default(),
        };

        assert_eq!(super::schema_service_name_for_publisher(&publisher), None);
    }

    #[test]
    fn topic_graph_fingerprint_uses_dynamic_schema_fingerprints() {
        let hash = ros_z::SchemaHash([4; 32]);
        let fingerprint =
            super::topic_graph_fingerprint_from_publishers(&[publisher_endpoint("talker", hash)]);

        assert_eq!(fingerprint.publishers.len(), 1);
        assert_eq!(fingerprint.publishers[0].topic, "/chatter");
        assert_eq!(fingerprint.publishers[0].node_namespace, "/");
        assert_eq!(fingerprint.publishers[0].node_name, "talker");
        assert_eq!(
            fingerprint.publishers[0].type_info,
            ros_z::TypeInfo::new("std_msgs::String", hash),
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn preinstall_dynamic_graph_change_discards_built_cache_after_marking_seen() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = context
            .create_node("dynamic_preinstall_guard")
            .without_schema_service()
            .build()
            .await
            .unwrap();
        let topic = "/dynamic_preinstall_guard_topic";
        let _publisher = node.publisher::<String>(topic).build().await.unwrap();
        let schema_service = node
            .service_server::<ros_z::dynamic::GetSchema>("~get_schema")
            .build()
            .await
            .unwrap();

        wait_for_publisher_count(&node, topic, 1).await;
        wait_for_service_count(&node, "/dynamic_preinstall_guard/get_schema", 1).await;
        let mut graph_revisions = node.graph().watch_revisions();
        graph_revisions.mark_seen();
        let build_fingerprint = super::dynamic_graph_fingerprint(&node, topic);

        drop(schema_service);
        wait_for_service_count(&node, "/dynamic_preinstall_guard/get_schema", 0).await;

        assert!(
            super::preinstall_dynamic_observing_graph_fingerprint(
                &mut graph_revisions,
                &node,
                topic,
                &build_fingerprint,
            )
            .is_none()
        );
    }

    #[test]
    fn retry_error_chain_does_not_duplicate_displayed_sources() {
        let error = TopicReference::new("bad%topic").unwrap_err();

        let message = super::format_error_chain(&error);

        assert_eq!(message.matches("invalid component 'bad%topic'").count(), 1);
    }

    #[test]
    fn retry_error_chain_keeps_distinct_suffix_source() {
        #[derive(Debug, thiserror::Error)]
        #[error("outer message mentions inner failure")]
        struct OuterError {
            #[source]
            source: InnerError,
        }

        #[derive(Debug, thiserror::Error)]
        #[error("inner failure")]
        struct InnerError;

        let message = super::format_error_chain(&OuterError { source: InnerError });

        assert_eq!(
            message,
            "outer message mentions inner failure: inner failure"
        );
    }

    #[test]
    fn build_inputs_changed_reports_desired_change() {
        let desired = DesiredObservation::new(
            TopicReference::new("status").unwrap(),
            RetentionPolicy::LatestOnly,
        );
        let (desired_sender, mut desired_receiver) = watch::channel(desired);
        let (_target_sender, mut target_receiver) =
            watch::channel(TargetIdentity::new("/").unwrap());
        let current_key =
            super::observation_build_key(&target_receiver.borrow(), &desired_receiver.borrow())
                .unwrap();

        assert!(matches!(
            super::build_inputs_changed(&mut desired_receiver, &mut target_receiver, &current_key),
            Ok(false)
        ));

        desired_sender.send_modify(|desired| {
            desired.reconnect_revision += 1;
        });

        assert!(matches!(
            super::build_inputs_changed(&mut desired_receiver, &mut target_receiver, &current_key),
            Ok(true)
        ));
    }

    #[test]
    fn build_inputs_changed_reports_closed_desired_channel() {
        let desired = DesiredObservation::new(
            TopicReference::new("status").unwrap(),
            RetentionPolicy::LatestOnly,
        );
        let (desired_sender, mut desired_receiver) = watch::channel(desired);
        let (_target_sender, mut target_receiver) =
            watch::channel(TargetIdentity::new("/").unwrap());
        let current_key =
            super::observation_build_key(&target_receiver.borrow(), &desired_receiver.borrow())
                .unwrap();

        drop(desired_sender);

        assert!(
            super::build_inputs_changed(&mut desired_receiver, &mut target_receiver, &current_key)
                .is_err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn topic_graph_fingerprint_ignores_same_node_same_schema_endpoint_churn() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = context
            .create_node("fingerprint_same_node_pub")
            .build()
            .await
            .unwrap();
        let topic = "/42/fingerprint_same_node";
        let _first_publisher = node.publisher::<String>(topic).build().await.unwrap();

        wait_for_publisher_count(&node, topic, 1).await;
        let first_fingerprint = super::topic_graph_fingerprint(&node, topic);

        let _second_publisher = node.publisher::<String>(topic).build().await.unwrap();
        wait_for_publisher_count(&node, topic, 2).await;
        let second_fingerprint = super::topic_graph_fingerprint(&node, topic);

        assert_eq!(
            first_fingerprint, second_fingerprint,
            "same-node same-schema endpoint churn should not be a relevant graph change"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn topic_graph_fingerprint_distinguishes_same_schema_different_publisher_node() {
        let context = ContextBuilder::default().build().await.unwrap();
        let observer_node = context
            .create_node("fingerprint_observer")
            .build()
            .await
            .unwrap();
        let first_node = context
            .create_node("fingerprint_first_pub")
            .build()
            .await
            .unwrap();
        let second_node = context
            .create_node("fingerprint_second_pub")
            .build()
            .await
            .unwrap();
        let topic = "/42/fingerprint_different_node";
        let _first_publisher = first_node.publisher::<String>(topic).build().await.unwrap();

        wait_for_publisher_count(&observer_node, topic, 1).await;
        let first_fingerprint = super::topic_graph_fingerprint(&observer_node, topic);

        let _second_publisher = second_node
            .publisher::<String>(topic)
            .build()
            .await
            .unwrap();
        wait_for_publisher_count(&observer_node, topic, 2).await;
        let second_fingerprint = super::topic_graph_fingerprint(&observer_node, topic);

        assert_ne!(
            first_fingerprint, second_fingerprint,
            "a new publisher node is a relevant graph change for dynamic schema discovery"
        );
    }

    #[test]
    fn observation_update_receiver_reports_lagged() {
        let (sender, receiver) = broadcast::channel(1);
        let mut receiver = TopicObservationUpdateReceiver::new(receiver);
        sender.send(TopicObservationUpdate::DataChanged).unwrap();
        sender.send(TopicObservationUpdate::DataChanged).unwrap();

        assert!(matches!(
            receiver.try_recv(),
            Ok(Some(TopicObservationUpdate::Lagged { dropped: 1 }))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn observing_rebuild_wait_forwards_data_changed_without_refreshing_status() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = context
            .create_node("data_changed_no_status_refresh")
            .build()
            .await
            .unwrap();
        let topic = "/data_changed_no_status_refresh";
        let stale_cache =
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample);
        let fresh_cache = CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::Ready);
        let cache_state = Arc::new(CachedSubscriptionState::<String>::new(
            fresh_cache,
            RetentionPolicy::LatestOnly,
        ));
        let (observation_update_sender, observation_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let observation_state = Arc::new(Mutex::new(super::TopicObservationState {
            status: TopicObservationStatus::Observing {
                cache: stale_cache.clone(),
            },
            display_cache: Some(cache_state.handle()),
            updates: Some(observation_update_sender),
        }));
        let state = Arc::downgrade(&observation_state);
        let mut observation_updates =
            TopicObservationUpdateReceiver::new(observation_update_receiver);

        let desired = DesiredObservation::new(
            TopicReference::new("data_changed_no_status_refresh").unwrap(),
            RetentionPolicy::LatestOnly,
        );
        let (_desired_sender, mut desired_receiver) = watch::channel(desired);
        let (_target_sender, mut target_receiver) =
            watch::channel(TargetIdentity::new("/").unwrap());
        let mut graph_revisions = node.graph().watch_revisions();
        let graph_fingerprint = super::topic_graph_fingerprint(&node, topic);
        let build_key =
            super::observation_build_key(&target_receiver.borrow(), &desired_receiver.borrow())
                .unwrap();
        let (cache_update_sender, cache_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let mut cache_updates = CachedSubscriptionUpdateReceiver::new(cache_update_receiver);

        let wait = super::wait_for_observing_rebuild(
            &state,
            (&node, topic, build_key, graph_fingerprint),
            &mut desired_receiver,
            &mut target_receiver,
            &mut graph_revisions,
            &mut cache_updates,
        );
        let send_cache_update = async {
            cache_update_sender
                .send(CachedSubscriptionUpdate::DataChanged)
                .unwrap();
            drop(cache_update_sender);
        };

        let (should_rebuild, _) = tokio::join!(wait, send_cache_update);

        assert!(should_rebuild);
        assert!(matches!(
            observation_updates.recv().await,
            Ok(TopicObservationUpdate::DataChanged)
        ));
        assert!(matches!(observation_updates.try_recv(), Ok(None)));
        assert_eq!(
            observation_state.lock().status,
            TopicObservationStatus::Observing { cache: stale_cache }
        );
    }

    #[test]
    fn refreshing_observing_status_reconciles_change_after_cache_update_subscription() {
        let stale_cache =
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample);
        let cache_state = Arc::new(CachedSubscriptionState::<String>::new(
            stale_cache.clone(),
            RetentionPolicy::LatestOnly,
        ));
        let cache = cache_state.handle();
        let (observation_update_sender, observation_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let observation_state = Arc::new(Mutex::new(super::TopicObservationState {
            status: TopicObservationStatus::Observing { cache: stale_cache },
            display_cache: Some(cache.clone()),
            updates: Some(observation_update_sender),
        }));
        let state = Arc::downgrade(&observation_state);
        let mut observation_updates =
            TopicObservationUpdateReceiver::new(observation_update_receiver);

        let mut cache_updates = super::subscribe_built_cache_updates(&cache).unwrap();
        cache_state.store_latest(string_sample_record("ready"));
        let fresh_cache = cache.status();

        assert!(super::refresh_observing_status(&state));
        assert!(matches!(
            cache_updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::StatusChanged(_)))
        ));
        assert!(matches!(
            cache_updates.try_recv(),
            Ok(Some(CachedSubscriptionUpdate::DataChanged))
        ));
        assert_eq!(
            observation_state.lock().status,
            TopicObservationStatus::Observing {
                cache: fresh_cache.clone()
            }
        );
        assert!(matches!(
            observation_updates.try_recv(),
            Ok(Some(TopicObservationUpdate::StatusChanged(
                TopicObservationStatus::Observing { cache }
            ))) if cache == fresh_cache
        ));
        assert!(matches!(observation_updates.try_recv(), Ok(None)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn installing_cache_with_retained_sample_emits_data_changed() {
        let cache_state = Arc::new(CachedSubscriptionState::<String>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        cache_state.store_latest(string_sample_record("ready before install"));
        let cache = cache_state.handle();
        let built_cache = cache.clone();
        let (observation_update_sender, observation_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let observation_state = Arc::new(Mutex::new(super::TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            updates: Some(observation_update_sender),
        }));
        let state = Arc::downgrade(&observation_state);
        let mut observation_updates =
            TopicObservationUpdateReceiver::new(observation_update_receiver);

        let mut cache_updates = super::subscribe_built_cache_updates(&cache).unwrap();
        assert!(matches!(cache_updates.try_recv(), Ok(None)));

        assert!(super::install_cache(&state, built_cache));

        assert!(matches!(
            observation_updates.try_recv(),
            Ok(Some(TopicObservationUpdate::StatusChanged(
                TopicObservationStatus::Observing { .. }
            )))
        ));
        assert!(matches!(
            observation_updates.try_recv(),
            Ok(Some(TopicObservationUpdate::DataChanged))
        ));
        assert!(matches!(observation_updates.try_recv(), Ok(None)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn subscribed_built_cache_forwards_data_changed_before_observing_wait_starts() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            context
                .create_node("prewait_data_changed_observer")
                .build()
                .await
                .unwrap(),
        );
        let topic = "/prewait_data_changed_observer";
        let cache_state = Arc::new(CachedSubscriptionState::<String>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let cache = cache_state.handle();
        let built_cache = cache.clone();
        let (observation_update_sender, observation_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let observation_state = Arc::new(Mutex::new(super::TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            updates: Some(observation_update_sender),
        }));
        let state = Arc::downgrade(&observation_state);
        let mut observation_updates =
            TopicObservationUpdateReceiver::new(observation_update_receiver);

        let mut cache_updates = super::subscribe_built_cache_updates(&built_cache).unwrap();
        assert!(super::install_cache(&state, built_cache));
        assert!(super::refresh_observing_status(&state));
        cache_state.store_latest(string_sample_record("ready"));

        let desired = DesiredObservation::new(
            TopicReference::new("prewait_data_changed_observer").unwrap(),
            RetentionPolicy::LatestOnly,
        );
        let (desired_sender, mut desired_receiver) = watch::channel(desired);
        let (_target_sender, mut target_receiver) =
            watch::channel(TargetIdentity::new("/").unwrap());
        let mut graph_revisions = node.graph().watch_revisions();
        let graph_fingerprint = super::topic_graph_fingerprint(&node, topic);
        let build_key =
            super::observation_build_key(&target_receiver.borrow(), &desired_receiver.borrow())
                .unwrap();

        let wait = super::wait_for_observing_rebuild(
            &state,
            (&node, topic, build_key, graph_fingerprint),
            &mut desired_receiver,
            &mut target_receiver,
            &mut graph_revisions,
            &mut cache_updates,
        );
        let observe_data_changed = async {
            loop {
                match observation_updates.recv().await {
                    Ok(TopicObservationUpdate::DataChanged) => break,
                    Ok(TopicObservationUpdate::StatusChanged(_)) => {}
                    Ok(TopicObservationUpdate::Lagged { dropped }) => {
                        panic!("observation update receiver lagged by {dropped}")
                    }
                    Err(_) => panic!("observation update receiver closed before DataChanged"),
                }
            }
            desired_sender.send_modify(|desired| {
                desired.reconnect_revision += 1;
            });
        };

        let (should_rebuild, ()) = tokio::join!(wait, observe_data_changed);

        assert!(should_rebuild);
        assert!(matches!(observation_updates.try_recv(), Ok(None)));
        assert!(
            observation_state
                .lock()
                .display_cache
                .as_ref()
                .and_then(|cache| cache.latest())
                .is_some()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn preinstall_graph_change_discards_built_cache_after_marking_seen() {
        let context = ContextBuilder::default().build().await.unwrap();
        let node = Arc::new(
            context
                .create_node("preinstall_graph_change_observer")
                .build()
                .await
                .unwrap(),
        );
        let topic = "/preinstall_graph_change_observer";
        let mut graph_revisions = node.graph().watch_revisions();
        graph_revisions.mark_seen();
        let target_fingerprint = super::topic_graph_fingerprint(&node, topic);
        let cache_state = Arc::new(CachedSubscriptionState::<String>::new(
            CachedSubscriptionStatusSnapshot::new(CachedSubscriptionStatus::WaitingForFirstSample),
            RetentionPolicy::LatestOnly,
        ));
        let built_cache = cache_state.handle();
        let (observation_update_sender, _observation_update_receiver) =
            broadcast::channel(super::UPDATE_BUFFER_CAPACITY);
        let observation_state = Arc::new(Mutex::new(super::TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            updates: Some(observation_update_sender),
        }));
        let state = Arc::downgrade(&observation_state);

        assert!(!super::topic_graph_fingerprint_changed_from(
            node.as_ref(),
            topic,
            &target_fingerprint,
        ));
        let _cache_updates = super::subscribe_built_cache_updates(&built_cache).unwrap();
        let _late_publisher = node.publisher::<String>(topic).build().await.unwrap();
        wait_for_publisher_count(&node, topic, 1).await;

        let should_install = super::preinstall_observing_graph_fingerprint(
            &mut graph_revisions,
            node.as_ref(),
            topic,
            &target_fingerprint,
        )
        .is_some();
        if should_install {
            assert!(super::install_cache(&state, built_cache));
        }

        assert!(
            !should_install,
            "late relevant graph changes should drop the built cache before install"
        );
        assert!(observation_state.lock().display_cache.is_none());
    }

    #[test]
    fn observation_update_receiver_reports_closed() {
        let (sender, receiver) = broadcast::channel(1);
        let mut receiver = TopicObservationUpdateReceiver::new(receiver);
        drop(sender);

        assert!(matches!(
            receiver.try_recv(),
            Err(TopicObservationUpdateClosed)
        ));
    }

    #[test]
    fn subscribe_updates_reports_closed_observation() {
        let observation = TopicObservation::<String>::new(DesiredObservation::new(
            TopicReference::new("status").unwrap(),
            RetentionPolicy::LatestOnly,
        ));
        observation.state.lock().status = TopicObservationStatus::Closed;

        assert!(matches!(
            observation.subscribe_updates(),
            Err(TopicObservationUpdateClosed)
        ));
    }

    #[test]
    fn retry_error_format_includes_source_chain() {
        #[derive(Debug, thiserror::Error)]
        #[error("outer failure")]
        struct OuterError {
            #[source]
            source: MiddleError,
        }

        #[derive(Debug, thiserror::Error)]
        #[error("middle failure")]
        struct MiddleError {
            #[source]
            source: InnerError,
        }

        #[derive(Debug, thiserror::Error)]
        #[error("inner failure")]
        struct InnerError;

        let error = OuterError {
            source: MiddleError { source: InnerError },
        };

        assert_eq!(
            super::format_error_chain(&error),
            "outer failure: middle failure: inner failure"
        );
    }

    #[test]
    fn retry_status_formats_error_source_chain() {
        #[derive(Debug, thiserror::Error)]
        #[error("outer failure")]
        struct OuterError {
            #[source]
            source: MiddleError,
        }

        #[derive(Debug, thiserror::Error)]
        #[error("middle failure")]
        struct MiddleError {
            #[source]
            source: InnerError,
        }

        #[derive(Debug, thiserror::Error)]
        #[error("inner failure")]
        struct InnerError;

        let observation = TopicObservation::<String>::new(DesiredObservation::new(
            TopicReference::new("status").unwrap(),
            RetentionPolicy::LatestOnly,
        ));
        let error = OuterError {
            source: MiddleError { source: InnerError },
        };

        assert!(super::set_retrying_status(
            &Arc::downgrade(&observation.state),
            &error,
        ));
        assert!(matches!(
            observation.status(),
            TopicObservationStatus::Retrying { error, .. }
                if error == "outer failure: middle failure: inner failure"
        ));
    }

    #[tokio::test]
    async fn dropping_last_observation_handle_stops_background_task() {
        let (exited_sender, exited_receiver) = oneshot::channel();
        let observation = test_observation_with_exit_signal::<String>(exited_sender);

        drop(observation);

        tokio::time::timeout(Duration::from_secs(1), exited_receiver)
            .await
            .expect("observation task should exit")
            .expect("exit signal should send");
    }

    fn test_observation_with_exit_signal<T: Send + Sync + 'static>(
        exited_sender: oneshot::Sender<()>,
    ) -> TopicObservation<T> {
        let observation = TopicObservation::new(DesiredObservation::new(
            TopicReference::new("status").unwrap(),
            RetentionPolicy::LatestOnly,
        ));
        let state = Arc::downgrade(&observation.state);

        tokio::spawn(async move {
            loop {
                if state.upgrade().is_none() {
                    let _ = exited_sender.send(());
                    break;
                }
                tokio::task::yield_now().await;
            }
        });

        observation
    }

    async fn wait_for_publisher_count(node: &ros_z::node::Node, topic: &str, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if node.graph().lock().publishers_on(topic).count() == expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("graph should observe publisher count");
    }

    async fn wait_for_service_count(node: &ros_z::node::Node, service: &str, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if node.graph().lock().services_named(service).count() == expected {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("graph should observe service count");
    }
}
