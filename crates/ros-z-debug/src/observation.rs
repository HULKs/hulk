use std::{
    future::Future,
    marker::PhantomData,
    sync::{Arc, Weak},
    time::Duration,
};

use parking_lot::Mutex;
use ros_z::{Message, dynamic::DynamicPayload, node::Node, time::Time};
use tokio::sync::{broadcast, watch};

use crate::{
    CachedSubscription, CachedSubscriptionFactory, CachedSubscriptionOptions,
    CachedSubscriptionStatusSnapshot, CachedSubscriptionUpdate, CachedSubscriptionUpdateReceiver,
    Error, JsonRenderPolicy, Result, RetentionPolicy, TargetIdentity, TopicReference,
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
    graph_changes: ros_z::graph::GraphChangeSubscription,
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
            graph_changes: self.inner.node.graph().subscribe_changes(),
        }
    }
}

/// Builder for a typed topic observation.
pub struct TopicObservationBuilder<T> {
    observer: TopicObserver,
    topic: TopicReference,
    retention: RetentionPolicy,
    _value: PhantomData<T>,
}

impl<T> TopicObservationBuilder<T> {
    fn new(observer: TopicObserver, topic: TopicReference) -> Self {
        Self {
            observer,
            topic,
            retention: RetentionPolicy::LatestOnly,
            _value: PhantomData,
        }
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
            namespace: None,
            node_name: None,
            retention: self.retention,
            reconnect_revision: 0,
        });
        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);
        let state = Arc::new(Mutex::new(TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            display_factory: None,
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
            namespace: None,
            node_name: None,
            retention: self.retention,
            reconnect_revision: 0,
        });
        let (updates, _) = broadcast::channel(UPDATE_BUFFER_CAPACITY);
        let state = Arc::new(Mutex::new(TopicObservationState {
            status: TopicObservationStatus::Building,
            display_cache: None,
            display_factory: None,
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
                display_factory: None,
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
            desired.node_name = Some(node_name);
        });
        Ok(())
    }

    /// Return this observation to inheriting node-name changes from its observer.
    pub fn inherit_node_name(&self) {
        self.controls.desired_sender.send_modify(|desired| {
            desired.node_name = None;
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
        self.state.lock().display_cache.as_ref()?.latest()
    }

    /// Return retained samples whose source time falls inside `[start, end]`.
    ///
    /// Observations with [`RetentionPolicy::LatestOnly`] return an empty window.
    pub fn window(&self, start: Time, end: Time) -> Vec<Arc<crate::SampleRecord<T>>> {
        self.state
            .lock()
            .display_cache
            .as_ref()
            .map_or_else(Vec::new, |cache| cache.window(start, end))
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
        self.latest_json_record().map(|record| record.value)
    }

    /// Render the latest retained dynamic payload as JSON with sample metadata.
    pub fn latest_json_record(&self) -> Option<crate::JsonSampleRecord> {
        self.inner
            .latest()
            .map(|record| dynamic_record_to_json(record, self.json_render_policy))
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON values.
    pub fn window_json(&self, start: Time, end: Time) -> Vec<serde_json::Value> {
        self.window_json_records(start, end)
            .into_iter()
            .map(|record| record.value)
            .collect()
    }

    /// Render retained dynamic payloads in `[start, end]` as JSON records.
    pub fn window_json_records(&self, start: Time, end: Time) -> Vec<crate::JsonSampleRecord> {
        self.inner
            .window(start, end)
            .into_iter()
            .map(|record| dynamic_record_to_json(record, self.json_render_policy))
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

fn dynamic_record_to_json(
    record: Arc<crate::SampleRecord<DynamicPayload>>,
    policy: JsonRenderPolicy,
) -> crate::JsonSampleRecord {
    crate::JsonSampleRecord {
        source_time: record.source_time,
        transport_time: record.transport_time,
        publication_id: record.publication_id,
        metadata: Arc::clone(&record.metadata),
        value: crate::dynamic_payload_to_json(&record.value, policy),
    }
}

struct TopicObservationState<T> {
    status: TopicObservationStatus,
    display_cache: Option<CachedSubscription<T>>,
    // Dropping a factory closes the subscriptions it built; retain it with the display cache.
    display_factory: Option<Arc<CachedSubscriptionFactory>>,
    updates: Option<broadcast::Sender<TopicObservationUpdate>>,
}

struct TopicObservationControls {
    desired_sender: watch::Sender<DesiredObservation>,
}

#[derive(Clone)]
struct DesiredObservation {
    topic: TopicReference,
    namespace: Option<String>,
    node_name: Option<String>,
    retention: RetentionPolicy,
    reconnect_revision: u64,
}

impl DesiredObservation {
    #[cfg(test)]
    fn new(topic: TopicReference, retention: RetentionPolicy) -> Self {
        Self {
            topic,
            namespace: None,
            node_name: None,
            retention,
            reconnect_revision: 0,
        }
    }
}

struct BuiltTypedCache<T> {
    cache: CachedSubscription<T>,
    factory: Arc<CachedSubscriptionFactory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TopicGraphFingerprint {
    publisher_types: Vec<(String, String)>,
}

struct ResolvedObservationTarget {
    identity: TargetIdentity,
    resolved_topic: String,
    graph_fingerprint: TopicGraphFingerprint,
}

fn topic_graph_fingerprint(node: &Node, resolved_topic: &str) -> TopicGraphFingerprint {
    let mut publisher_types = node
        .graph()
        .view()
        .publishers_on(resolved_topic)
        .into_iter()
        .map(|publisher| {
            (
                publisher.type_info.name,
                publisher.type_info.hash.to_hash_string(),
            )
        })
        .collect::<Vec<_>>();
    publisher_types.sort();
    TopicGraphFingerprint { publisher_types }
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
    if let Some(node_name) = desired
        .node_name
        .as_deref()
        .or_else(|| observer_identity.node_name())
    {
        identity.set_node_name(node_name)?;
    }
    Ok(identity)
}

async fn build_typed_cache<T>(
    node: Arc<Node>,
    schema_discovery_timeout: Duration,
    identity: TargetIdentity,
    desired: DesiredObservation,
) -> Result<BuiltTypedCache<T>>
where
    T: Message + Send + Sync + 'static,
    T::Codec: Send + Sync,
{
    let mut options = CachedSubscriptionOptions::with_target_namespace(identity.namespace())?;
    if let Some(node_name) = identity.node_name() {
        options.set_target_node_name(node_name)?;
    }
    options.set_schema_discovery_timeout(schema_discovery_timeout);

    let factory = Arc::new(CachedSubscriptionFactory::new(node, options));
    let cache = factory
        .subscribe_typed::<T>(desired.topic.as_str())?
        .retention(desired.retention)
        .build()
        .await?;
    Ok(BuiltTypedCache { cache, factory })
}

async fn build_dynamic_cache(
    node: Arc<Node>,
    schema_discovery_timeout: Duration,
    identity: TargetIdentity,
    desired: DesiredObservation,
) -> Result<BuiltTypedCache<DynamicPayload>> {
    let mut options = CachedSubscriptionOptions::with_target_namespace(identity.namespace())?;
    if let Some(node_name) = identity.node_name() {
        options.set_target_node_name(node_name)?;
    }
    options.set_schema_discovery_timeout(schema_discovery_timeout);

    let factory = Arc::new(CachedSubscriptionFactory::new(node, options));
    let cache = factory
        .subscribe_dynamic(desired.topic.as_str())?
        .retention(desired.retention)
        .build()
        .await?;
    Ok(BuiltTypedCache { cache, factory })
}

enum BuildWait<T> {
    Completed(Result<BuiltTypedCache<T>>),
    RebuildRequested,
    Closed,
}

async fn wait_for_build<T, F>(
    build: F,
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
) -> BuildWait<T>
where
    F: Future<Output = Result<BuiltTypedCache<T>>>,
{
    tokio::select! {
        result = build => BuildWait::Completed(result),
        result = desired_receiver.changed() => {
            if result.is_ok() {
                BuildWait::RebuildRequested
            } else {
                BuildWait::Closed
            }
        }
        result = target_receiver.changed() => {
            if result.is_ok() {
                BuildWait::RebuildRequested
            } else {
                BuildWait::Closed
            }
        }
    }
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
        task.graph_changes.mark_seen();
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
                if !set_retrying_status(&state, error.to_string()) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
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
        match wait_for_build(build, &mut desired_receiver, &mut task.target_receiver).await {
            BuildWait::Completed(Ok(built_cache)) => {
                let cache = built_cache.cache.clone();
                if !install_typed_cache(&state, built_cache) {
                    return;
                }
                task.graph_changes.mark_seen();
                let graph_fingerprint =
                    topic_graph_fingerprint(task.node.as_ref(), target.resolved_topic.as_str());
                let Ok(mut cache_updates) = cache.subscribe_updates() else {
                    continue;
                };

                if !wait_for_observing_rebuild(
                    &state,
                    (
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        graph_fingerprint,
                    ),
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
                    &mut cache_updates,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::Completed(Err(error)) => {
                if !set_retrying_status(&state, error.to_string()) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
                    Some((
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        target.graph_fingerprint,
                    )),
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
        task.graph_changes.mark_seen();
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
                if !set_retrying_status(&state, error.to_string()) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
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

        let build = build_dynamic_cache(
            Arc::clone(&task.node),
            task.schema_discovery_timeout,
            target.identity,
            desired,
        );
        match wait_for_build(build, &mut desired_receiver, &mut task.target_receiver).await {
            BuildWait::Completed(Ok(built_cache)) => {
                let cache = built_cache.cache.clone();
                if !install_typed_cache(&state, built_cache) {
                    return;
                }
                task.graph_changes.mark_seen();
                let graph_fingerprint =
                    topic_graph_fingerprint(task.node.as_ref(), target.resolved_topic.as_str());
                let Ok(mut cache_updates) = cache.subscribe_updates() else {
                    continue;
                };

                if !wait_for_observing_rebuild(
                    &state,
                    (
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        graph_fingerprint,
                    ),
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
                    &mut cache_updates,
                )
                .await
                {
                    close_observation(&state);
                    return;
                }
            }
            BuildWait::Completed(Err(error)) => {
                if !set_retrying_status(&state, error.to_string()) {
                    return;
                }
                if !wait_for_retry_signal(
                    task.retry_delay,
                    &mut desired_receiver,
                    &mut task.target_receiver,
                    &mut task.graph_changes,
                    Some((
                        task.node.as_ref(),
                        target.resolved_topic.as_str(),
                        target.graph_fingerprint,
                    )),
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
    let graph_fingerprint = topic_graph_fingerprint(node, &resolved_topic);
    Ok(ResolvedObservationTarget {
        identity,
        resolved_topic,
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
    graph_changes: &mut ros_z::graph::GraphChangeSubscription,
    graph_filter: Option<(&Node, &str, TopicGraphFingerprint)>,
) -> bool {
    let mut graph_filter = graph_filter;
    let retry_sleep = tokio::time::sleep(retry_delay);
    tokio::pin!(retry_sleep);
    loop {
        tokio::select! {
            _ = &mut retry_sleep => return true,
            result = desired_receiver.changed() => return result.is_ok(),
            result = target_receiver.changed() => return result.is_ok(),
            revision = graph_changes.changed() => {
                if revision.is_none() {
                    return false;
                }
                if let Some((node, resolved_topic, fingerprint)) = &mut graph_filter
                    && topic_graph_fingerprint_changed(node, resolved_topic, fingerprint)
                {
                    return true;
                }
            }
        }
    }
}

async fn wait_for_observing_rebuild<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    graph_filter: (&Node, &str, TopicGraphFingerprint),
    desired_receiver: &mut watch::Receiver<DesiredObservation>,
    target_receiver: &mut watch::Receiver<TargetIdentity>,
    graph_changes: &mut ros_z::graph::GraphChangeSubscription,
    cache_updates: &mut CachedSubscriptionUpdateReceiver,
) -> bool {
    let (node, resolved_topic, mut graph_fingerprint) = graph_filter;
    loop {
        let rebuild = tokio::select! {
            result = desired_receiver.changed() => return result.is_ok(),
            result = target_receiver.changed() => return result.is_ok(),
            revision = graph_changes.changed() => {
                if revision.is_none() {
                    return false;
                }
                topic_graph_fingerprint_changed(node, resolved_topic, &mut graph_fingerprint)
            }
            update = cache_updates.recv() => {
                match update {
                    Ok(CachedSubscriptionUpdate::DataChanged) => {
                        send_observation_update(
                            state,
                            TopicObservationUpdate::DataChanged,
                        );
                        !refresh_observing_status(state)
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
        state
            .lock()
            .display_cache
            .as_ref()
            .map(CachedSubscription::status)
    })
}

fn freeze_display_cache<T>(state: &Weak<Mutex<TopicObservationState<T>>>) -> bool {
    let Some(state) = state.upgrade() else {
        return false;
    };
    let (cache, factory) = {
        let mut state = state.lock();
        if state.updates.is_none() {
            return false;
        }
        (state.display_cache.clone(), state.display_factory.take())
    };
    drop(factory);
    if let Some(cache) = cache {
        cache.close_retaining_samples();
    }
    true
}

fn set_rebuild_status<T>(state: &Weak<Mutex<TopicObservationState<T>>>) -> bool {
    let status = match previous_cache(state) {
        Some(previous_cache) => TopicObservationStatus::Rebuilding { previous_cache },
        None => TopicObservationStatus::Building,
    };
    set_observation_status(state, status)
}

fn set_retrying_status<T>(state: &Weak<Mutex<TopicObservationState<T>>>, error: String) -> bool {
    let previous_cache = previous_cache(state);
    if !freeze_display_cache(state) {
        return false;
    }
    set_observation_status(
        state,
        TopicObservationStatus::Retrying {
            previous_cache,
            error,
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

fn install_typed_cache<T>(
    state: &Weak<Mutex<TopicObservationState<T>>>,
    built_cache: BuiltTypedCache<T>,
) -> bool {
    let status = TopicObservationStatus::Observing {
        cache: built_cache.cache.status(),
    };
    let Some(state) = state.upgrade() else {
        return false;
    };
    let updates = {
        let mut state = state.lock();
        if state.updates.is_none() {
            return false;
        }
        let status_changed = state.status != status;
        state.display_factory = Some(built_cache.factory);
        state.display_cache = Some(built_cache.cache);
        state.status = status.clone();
        status_changed.then(|| state.updates.clone()).flatten()
    };
    if let Some(updates) = updates {
        let _ = updates.send(TopicObservationUpdate::StatusChanged(status));
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
    let display_factory = {
        let mut state = state.lock();
        state.status = TopicObservationStatus::Closed;
        state.updates.take();
        state.display_factory.take()
    };
    drop(display_factory);
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

    use tokio::sync::{broadcast, oneshot};

    use super::{
        DesiredObservation, TopicObservation, TopicObservationBlockReason, TopicObservationStatus,
        TopicObservationUpdate, TopicObservationUpdateClosed, TopicObservationUpdateReceiver,
    };
    use crate::{RetentionPolicy, TargetIdentity, TopicReference};

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
            node_name: None,
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
            node_name: Some("vision".to_string()),
            retention: RetentionPolicy::LatestOnly,
            reconnect_revision: 0,
        };

        let effective = super::effective_identity(&observer_identity, &desired).unwrap();

        assert_eq!(effective.namespace(), "/99");
        assert_eq!(effective.node_name(), Some("vision"));
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
}
