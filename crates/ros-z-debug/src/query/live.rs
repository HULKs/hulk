use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    num::NonZeroUsize,
    sync::{Arc, Weak},
    time::Duration,
};

use futures_util::{
    StreamExt,
    stream::{self, BoxStream, SelectAll},
};
use parking_lot::Mutex;
use ros_z::{node::Node, pubsub::Received};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::plan::{Evaluation, QueryPlan, SourceState, SourceView, State, TopicSource};
use super::*;
use crate::{
    ObservationPolicy, RetentionPolicy, builder::subscriber_qos, observation::format_error_chain,
};

#[derive(Clone, Copy)]
/// Per-feed retained-payload budgets, keyed by topic and subscription policy.
/// Both limits apply; oldest arrivals are evicted regardless of projected time.
/// Query windows cannot recover evicted samples. Multiple feeds multiply budgets.
pub struct LiveRetention {
    pub samples: NonZeroUsize,
    pub bytes: NonZeroUsize,
}

impl Default for LiveRetention {
    fn default() -> Self {
        Self {
            samples: NonZeroUsize::new(4096).unwrap(),
            bytes: NonZeroUsize::new(128 * 1024 * 1024).unwrap(),
        }
    }
}

struct Index {
    users: usize,
    cancel: CancellationToken,
    error: Option<String>,
    last_rejection: Option<String>,
    building: bool,
    records: BTreeMap<(Time, u64), Arc<RawRecord>>,
    times: HashMap<u64, Time>,
    snapshot: Option<LiveRecords>,
}

impl Drop for Index {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl Index {
    fn new(users: usize, error: Option<String>, building: bool) -> Self {
        Self {
            users,
            error,
            building,
            cancel: CancellationToken::new(),
            last_rejection: None,
            records: BTreeMap::new(),
            times: HashMap::new(),
            snapshot: None,
        }
    }
    fn insert(&mut self, record: RawRecord) {
        self.snapshot = None;
        self.times.insert(record.id, record.time);
        self.records
            .insert((record.time, record.id), Arc::new(record));
    }
    fn remove(&mut self, id: u64) {
        if let Some(time) = self.times.remove(&id) {
            self.snapshot = None;
            self.records.remove(&(time, id));
        }
    }

    fn snapshot(&mut self) -> LiveRecords {
        Arc::clone(
            self.snapshot
                .get_or_insert_with(|| Arc::new(self.records.values().cloned().collect())),
        )
    }
}

struct ProjectionLease {
    feed: Arc<Feed>,
    projection: TimeProjection,
}

impl Drop for ProjectionLease {
    fn drop(&mut self) {
        let mut state = self.feed.state.lock();
        if let Some(index) = state.indexes.get_mut(&self.projection) {
            index.users -= 1;
            if index.users == 0 {
                state.indexes.remove(&self.projection);
            }
        }
    }
}

#[derive(Default)]
struct FeedState {
    info: Option<TopicInfo>,
    records: VecDeque<RawRecord>,
    indexes: HashMap<TimeProjection, Index>,
    bytes: usize,
    next_id: u64,
    limited: bool,
    error: Option<String>,
}

impl FeedState {
    fn ingest(&mut self, sample: Received<Arc<[u8]>>, retention: LiveRetention) {
        let record = RawRecord {
            publication: Some((sample.source_global_id.into_bytes(), sample.sequence_number)),
            id: self.next_id,
            time: sample.source_time,
            source_time: sample.source_time,
            log_time: sample.transport_time,
            data: sample.message,
        };
        self.next_id = self.next_id.wrapping_add(1);
        let schema = self.info.as_ref().and_then(|info| info.schema.as_ref());
        for (projection, index) in &mut self.indexes {
            if index.error.is_some()
                || (schema.is_none() && *projection != TimeProjection::SourceTime)
            {
                continue;
            }
            match project_record(&record, projection, schema) {
                Ok(time) => index.insert(RawRecord {
                    time,
                    ..record.clone()
                }),
                Err(error) => index.last_rejection = Some(error.to_string()),
            }
        }
        self.bytes += record.data.len();
        self.records.push_back(record);
        while self.records.len() > retention.samples.get() || self.bytes > retention.bytes.get() {
            let record = self
                .records
                .pop_front()
                .expect("retained payload exceeds its budget");
            self.bytes -= record.data.len();
            self.limited = true;
            for index in self.indexes.values_mut() {
                index.remove(record.id);
            }
        }
    }
}

struct Feed {
    state: Mutex<FeedState>,
    changed: watch::Sender<u64>,
    cancel: CancellationToken,
}

impl Drop for Feed {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

impl Feed {
    fn projection(self: &Arc<Self>, projection: &TimeProjection) -> ProjectionLease {
        let mut state = self.state.lock();
        if !state.indexes.contains_key(projection) {
            self.build_index(&mut state, projection, 0);
        }
        state
            .indexes
            .get_mut(projection)
            .expect("registered projection")
            .users += 1;
        ProjectionLease {
            feed: Arc::clone(self),
            projection: projection.clone(),
        }
    }

    fn build_index(
        self: &Arc<Self>,
        state: &mut FeedState,
        projection: &TimeProjection,
        users: usize,
    ) {
        let records: Vec<_> = state.records.iter().cloned().collect();
        let schema = state.info.as_ref().and_then(|info| info.schema.clone());
        let needs_schema = *projection != TimeProjection::SourceTime;
        let index = Index::new(
            users,
            if needs_schema {
                schema.as_ref().map_or_else(
                    || state.info.as_ref().and_then(|info| info.error.clone()),
                    |schema| projection.validate(schema).err().map(|e| e.to_string()),
                )
            } else {
                None
            },
            !records.is_empty() || (needs_schema && schema.is_none()),
        );
        let cancel = index.cancel.clone();
        let valid = index.error.is_none();
        state.indexes.insert(projection.clone(), index);
        if !records.is_empty() && valid && (!needs_schema || schema.is_some()) {
            let projection = projection.clone();
            let weak = Arc::downgrade(self);
            let changed = self.changed.clone();
            tokio::task::spawn_blocking(move || {
                let mut indexed = Vec::with_capacity(records.len());
                let mut issue = None;
                for mut record in records {
                    if cancel.is_cancelled() {
                        return;
                    }
                    match project_record(&record, &projection, schema.as_ref()) {
                        Ok(time) => {
                            record.time = time;
                            indexed.push(record);
                        }
                        Err(error) => issue = Some(error.to_string()),
                    }
                }
                if let Some(feed) = weak.upgrade() {
                    let mut state = feed.state.lock();
                    // A dropped/recreated projection or reconnect cancels this generation.
                    if cancel.is_cancelled() {
                        return;
                    }
                    let first = state.records.front().map(|record| record.id);
                    let index = state
                        .indexes
                        .get_mut(&projection)
                        .expect("active projection");
                    for record in indexed {
                        if first.is_some_and(|first| record.id >= first) {
                            index.insert(record);
                        }
                    }
                    index.building = false;
                    if issue.is_some() {
                        index.last_rejection = issue;
                    }
                }
                changed.send_modify(|revision| *revision = revision.wrapping_add(1));
            });
        }
    }
}

fn project_record(
    record: &RawRecord,
    projection: &TimeProjection,
    schema: Option<&Schema>,
) -> Result<Time> {
    if *projection == TimeProjection::SourceTime {
        return Ok(record.source_time);
    }
    projection
        .project_cdr(
            record.source_time,
            &record.data,
            schema.ok_or_else(|| query_error("topic schema unavailable"))?,
        )
        .map_err(query_error)
}

#[derive(Clone)]
pub struct Live {
    node: Arc<Node>,
    retention: LiveRetention,
    feeds: Arc<Mutex<FeedRegistry>>,
    topics: TopicList,
}

type FeedRegistry = HashMap<(String, ObservationPolicy), Weak<Feed>>;
type FeedUpdates = SelectAll<BoxStream<'static, ()>>;
type LiveRecords = Arc<Vec<Arc<RawRecord>>>;

fn advertised_topics(node: &Node) -> Vec<TopicInfo> {
    let mut topics: Vec<_> = node
        .graph()
        .lock()
        .publishers()
        .map(|publisher| TopicInfo {
            name: publisher.topic.clone(),
            type_info: publisher.type_info.clone(),
            schema: None,
            error: None,
        })
        .collect();
    topics.sort_by(|a, b| {
        (&a.name, &a.type_info.name, a.type_info.hash.0).cmp(&(
            &b.name,
            &b.type_info.name,
            b.type_info.hash.0,
        ))
    });
    topics.dedup_by(|a, b| a.name == b.name && a.type_info == b.type_info);
    topics
}

impl Live {
    pub fn new(node: Arc<Node>, retention: LiveRetention) -> Self {
        let (sender, receiver) = watch::channel(Arc::new(TopicSnapshot {
            loading: true,
            ..Default::default()
        }));
        let discovery_node = Arc::clone(&node);
        tokio::spawn(async move {
            let mut revisions = discovery_node.graph().watch_revisions();
            loop {
                revisions.mark_seen();
                sender.send_replace(Arc::new(TopicSnapshot {
                    topics: advertised_topics(&discovery_node),
                    loading: false,
                    error: None,
                }));
                tokio::select! {
                    _ = sender.closed() => return,
                    revision = revisions.changed() => if revision.is_none() { return; },
                }
            }
        });
        Self {
            node,
            retention,
            feeds: Arc::new(Mutex::new(HashMap::new())),
            topics: TopicList(receiver),
        }
    }

    // Query plans resolve and validate source topics before acquiring feeds.
    fn feed(&self, source: &TopicSource) -> ProjectionLease {
        let policy = source
            .live_policy
            .with_retention(RetentionPolicy::LatestOnly)
            .with_update_buffer_capacity(ObservationPolicy::default().update_buffer_capacity());
        let key = (source.topic.clone(), policy);
        let mut feeds = self.feeds.lock();
        feeds.retain(|_, feed| feed.strong_count() > 0);
        let feed = feeds.get(&key).and_then(Weak::upgrade).unwrap_or_else(|| {
            let feed = Arc::new(Feed {
                changed: watch::channel(0).0,
                state: Mutex::new(FeedState::default()),
                cancel: CancellationToken::new(),
            });
            feeds.insert(key, Arc::downgrade(&feed));
            tokio::spawn(receive(
                Arc::clone(&self.node),
                source.topic.clone(),
                Arc::downgrade(&feed),
                feed.cancel.clone(),
                feed.changed.clone(),
                self.retention,
                policy,
            ));
            feed
        });
        drop(feeds);
        feed.projection(&source.projection)
    }
}

impl Backend for Live {
    fn topics(&self) -> TopicList {
        self.topics.clone()
    }

    fn observe(&self, cursor: CursorHandle, mut plan: QueryPlan) -> Result<PlanObservation> {
        plan.resolve_topics(self.node.namespace())?;
        let query = Arc::new(LiveQuery {
            live: self.clone(),
            sources: plan.sources()?,
            plan,
            cursor: cursor.clone(),
            leases: Mutex::new(Vec::new()),
            cache: Mutex::new(None),
        });
        let changes = query
            .refresh_sources(&self.topics.snapshot())
            .unwrap_or_default();
        let (sender, updates) = watch::channel(0);
        let reader = Arc::clone(&query);
        let observation = PlanObservation::new(cursor, move || reader.snapshot(), updates);
        let cancel = observation.cancellation_token();
        tokio::spawn(query.run(sender, changes, cancel));
        Ok(observation)
    }
}

struct LiveQuery {
    live: Live,
    plan: QueryPlan,
    sources: Vec<TopicSource>,
    cursor: CursorHandle,
    leases: Mutex<Vec<Option<ProjectionLease>>>,
    cache: Mutex<Option<CachedPlan>>,
}

struct CachedPlan {
    revisions: Vec<Option<u64>>,
    catalog: Arc<TopicSnapshot>,
    snapshot: Arc<PlanSnapshot>,
}

impl LiveQuery {
    fn refresh_sources(&self, catalog: &TopicSnapshot) -> Option<FeedUpdates> {
        let active = self.plan.active_sources(catalog).expect("validated plan");
        let mut leases = self.leases.lock();
        if leases.len() == active.len()
            && leases
                .iter()
                .map(Option::is_some)
                .eq(active.iter().copied())
        {
            return None;
        }
        leases.resize_with(active.len(), || None);
        for ((lease, source), active) in leases.iter_mut().zip(&self.sources).zip(active) {
            if active && lease.is_none() {
                *lease = Some(self.live.feed(source));
            } else if !active {
                *lease = None;
            }
        }
        Some(stream::select_all(leases.iter().flatten().map(|lease| {
            stream::unfold(lease.feed.changed.subscribe(), |mut updates| async move {
                updates.changed().await.ok()?;
                Some(((), updates))
            })
            .boxed()
        })))
    }

    fn snapshot(&self) -> Arc<PlanSnapshot> {
        let position = self.cursor.position();
        let time = match position.cursor {
            Cursor::Live => self.live.node.clock().now(),
            Cursor::At(time) => time,
        };
        let catalog = self.live.topics.snapshot();
        let leases = self.leases.lock();
        let revisions: Vec<_> = leases
            .iter()
            .map(|lease| lease.as_ref().map(|lease| *lease.feed.changed.borrow()))
            .collect();
        let mut cache = self.cache.lock();
        if let Some(cached) = &*cache
            && cached.revisions == revisions
            && Arc::ptr_eq(&cached.catalog, &catalog)
            && cached.snapshot.revision == position.revision
            && cached.snapshot.evaluation.valid_for.contains(time)
        {
            return Arc::clone(&cached.snapshot);
        }
        let mut retained = vec![None; self.sources.len()];
        let views: Vec<_> = self
            .sources
            .iter()
            .zip(leases.iter())
            .zip(&mut retained)
            .map(|((source, lease), records)| {
                capture_source(source, lease.as_ref(), &catalog, records)
            })
            .collect();
        let evaluation = self
            .plan
            .evaluate(time, &views, |source, slot| {
                Ok(Arc::clone(
                    &retained[source].as_ref().expect("selected source is ready")[slot],
                ))
            })
            .unwrap_or_else(|error| Evaluation::unresolved(time, State::Failed(error.to_string())));
        let snapshot = Arc::new(PlanSnapshot {
            revision: position.revision,
            evaluation: Arc::new(evaluation),
        });
        *cache = Some(CachedPlan {
            revisions,
            catalog,
            snapshot: Arc::clone(&snapshot),
        });
        snapshot
    }

    async fn run(
        self: Arc<Self>,
        sender: watch::Sender<u64>,
        mut changes: FeedUpdates,
        cancel: CancellationToken,
    ) {
        let mut positions = self.cursor.subscribe();
        let mut catalogs = self.live.topics.subscribe_updates();
        loop {
            positions.borrow_and_update();
            let catalog = catalogs.borrow_and_update().clone();
            if let Some(updated) = self.refresh_sources(&catalog) {
                changes = updated;
            }
            sender.send_modify(|revision| *revision = revision.wrapping_add(1));
            tokio::select! {
                _ = cancel.cancelled() => break,
                changed = positions.changed() => if changed.is_err() { break; },
                _ = changes.next(), if !changes.is_empty() => {},
                _ = catalogs.changed() => {},
                _ = tokio::time::sleep(Duration::from_millis(20)), if self.cursor.position().cursor == Cursor::Live => {},
            }
        }
    }
}

fn capture_source(
    source: &TopicSource,
    lease: Option<&ProjectionLease>,
    catalog: &TopicSnapshot,
    retained: &mut Option<LiveRecords>,
) -> SourceState {
    if !catalog.topics.iter().any(|info| info.name == source.topic) {
        return if catalog.loading {
            SourceState::Pending
        } else {
            SourceState::Absent
        };
    }
    let Some(lease) = lease else {
        return SourceState::Pending;
    };
    let mut state = lease.feed.state.lock();
    let index = &state.indexes[&lease.projection];
    if let Some(error) = &index.error {
        return SourceState::Failed(error.clone());
    }
    if index.building {
        return SourceState::Pending;
    }
    let Some(info) = &state.info else {
        return state.error.as_ref().map_or(SourceState::Pending, |error| {
            SourceState::Failed(error.clone())
        });
    };
    if catalog
        .topics
        .iter()
        .any(|topic| topic.name == source.topic && topic.type_info != info.type_info)
    {
        return SourceState::Failed("incompatible publisher schema".into());
    }
    let topic = info.clone();
    let coverage = Coverage {
        source: if state.limited {
            SourceCoverage::RetentionLimited
        } else {
            SourceCoverage::Live
        },
        truncated: false,
    };
    let diagnostics = state
        .error
        .iter()
        .chain(info.error.iter())
        .chain(index.last_rejection.iter())
        .cloned()
        .collect();
    let records = state
        .indexes
        .get_mut(&lease.projection)
        .expect("leased projection")
        .snapshot();
    *retained = Some(Arc::clone(&records));
    SourceState::Ready(SourceView {
        topic,
        index: records,
        coverage,
        diagnostics,
    })
}

impl plan::TimeIndex for Vec<Arc<RawRecord>> {
    fn len(&self) -> usize {
        self.len()
    }
    fn time(&self, index: usize) -> Time {
        self[index].time
    }
}

async fn receive(
    node: Arc<Node>,
    topic: String,
    feed: Weak<Feed>,
    cancel: CancellationToken,
    changed: watch::Sender<u64>,
    retention: LiveRetention,
    policy: ObservationPolicy,
) {
    loop {
        let result = tokio::select! {
            _ = cancel.cancelled() => return,
            result = receive_connection(&node, &topic, &feed, &changed, retention, policy) => result,
        };
        if let Some(feed) = feed.upgrade() {
            feed.state.lock().error = Some(
                result
                    .err()
                    .map_or_else(|| "publisher changed".into(), |e| format_error_chain(&e)),
            );
        }
        changed.send_modify(|revision| *revision = revision.wrapping_add(1));
        tokio::select! { _ = cancel.cancelled() => return, _ = tokio::time::sleep(Duration::from_secs(1)) => {} }
    }
}

async fn receive_connection(
    node: &Node,
    topic: &str,
    feed: &Weak<Feed>,
    changed: &watch::Sender<u64>,
    retention: LiveRetention,
    policy: ObservationPolicy,
) -> Result<()> {
    let mut revisions = node.graph().watch_revisions();
    revisions.mark_seen();
    let publishers = || {
        let mut publishers = node
            .graph()
            .lock()
            .publishers_on(topic)
            .map(|p| {
                (
                    ros_z::EndpointGlobalId::from(p).into_bytes(),
                    p.type_info.clone(),
                )
            })
            .collect::<Vec<_>>();
        publishers.sort_by_key(|(id, _)| *id);
        publishers
    };
    let fingerprint = publishers();
    let Some((_, type_info)) = fingerprint.first() else {
        return Ok(());
    };
    if fingerprint.iter().any(|(_, info)| info != type_info) {
        return Err(query_error("incompatible publisher schema"));
    }
    let mut subscriber = node
        .dynamic_raw_subscriber(topic, type_info.clone())
        .qos(subscriber_qos(node, topic, policy))
        .queue_overflow_reporting(policy.queue_overflow_reporting());
    if let Some(capacity) = policy.subscriber_queue_capacity() {
        subscriber = subscriber.queue_capacity(capacity);
    }
    let mut subscriber = subscriber.build().await?;
    if publishers() != fingerprint {
        return Ok(());
    }
    if let Some(feed) = feed.upgrade() {
        let mut state = feed.state.lock();
        state.info = Some(TopicInfo {
            name: topic.into(),
            type_info: type_info.clone(),
            schema: None,
            error: None,
        });
        state.records.clear();
        state.bytes = 0;
        state.error = None;
        state.limited = false;
        for (projection, index) in &mut state.indexes {
            *index = Index::new(index.users, None, *projection != TimeProjection::SourceTime);
        }
    }
    changed.send_modify(|revision| *revision = revision.wrapping_add(1));
    // Schema discovery enriches raw feeds; source-time subscriptions never wait for it.
    let discovery = discover_schema(node, topic, feed, changed, type_info);
    tokio::pin!(discovery);
    let mut discovering = true;
    loop {
        let sample = tokio::select! {
            sample = subscriber.recv() => sample?,
            _ = &mut discovery, if discovering => { discovering = false; continue; },
            revision = revisions.changed() => { if revision.is_none() || publishers() != fingerprint { return Ok(()); } continue; }
        };
        let Some(feed) = feed.upgrade() else {
            return Ok(());
        };
        match Received::try_from_sample(&sample, sample.payload().to_bytes().as_ref().into()) {
            Ok(sample) => feed.state.lock().ingest(sample, retention),
            Err(error) => feed.state.lock().error = Some(format_error_chain(&error)),
        }
        changed.send_modify(|revision| *revision = revision.wrapping_add(1));
        // A continuously ready raw queue must not monopolize a runtime worker.
        tokio::task::yield_now().await;
    }
}

async fn discover_schema(
    node: &Node,
    topic: &str,
    feed: &Weak<Feed>,
    changed: &watch::Sender<u64>,
    type_info: &ros_z::TypeInfo,
) {
    loop {
        let result = node
            .discover_topic_schema(topic, Duration::from_secs(3))
            .await;
        {
            let Some(feed) = feed.upgrade() else { return };
            let mut state = feed.state.lock();
            let Some(info) = &mut state.info else { return };
            if &info.type_info != type_info {
                return;
            }
            match &result {
                Ok(discovered) if discovered.type_info() == *type_info => {
                    info.schema = Some(discovered.schema.clone());
                    info.error = None;
                }
                Ok(_) => return,
                Err(error) => info.error = Some(format_error_chain(error)),
            }
            let projections: Vec<_> = state
                .indexes
                .iter()
                .filter(|(projection, _)| **projection != TimeProjection::SourceTime)
                .map(|(projection, index)| (projection.clone(), index.users))
                .collect();
            for (projection, users) in projections {
                feed.build_index(&mut state, &projection, users);
            }
        }
        changed.send_modify(|revision| *revision = revision.wrapping_add(1));
        if result.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::{Message, message::WireEncoder};
    use tokio::sync::oneshot;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn projection_leases_release_unused_indexes() {
        let context = ros_z::context::ContextBuilder::default()
            .build()
            .await
            .unwrap();
        let node = Arc::new(
            context
                .create_node("projection_leases")
                .build()
                .await
                .unwrap(),
        );
        let live = Live::new(node, LiveRetention::default());
        let a = live.feed(&TopicSource::new("/topic", TimeProjection::SourceTime));
        let b = live.feed(&TopicSource::new(
            "/topic",
            TimeProjection::field(".stamp", TimeEncoding::RosZTime).unwrap(),
        ));
        assert!(Arc::ptr_eq(&a.feed, &b.feed));
        assert_eq!(a.feed.state.lock().indexes.len(), 2);
        drop(b);
        assert_eq!(a.feed.state.lock().indexes.len(), 1);
        let weak = Arc::downgrade(&a.feed);
        drop(a);
        tokio::time::timeout(Duration::from_secs(1), async {
            while weak.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[test]
    fn late_backfill_respects_eviction_and_replaced_projection_generations() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (started, waiting) = oneshot::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let blocker = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                blocked.recv_timeout(Duration::from_secs(5)).unwrap();
            });
            waiting.await.unwrap();
            let schema = Arc::new(Time::schema());
            let feed = Arc::new(Feed {
                state: Mutex::new(FeedState {
                    info: Some(TopicInfo {
                        name: "/topic".into(),
                        type_info: Time::type_info(),
                        schema: Some(schema.clone()),
                        error: None,
                    }),
                    ..Default::default()
                }),
                changed: watch::channel(0).0,
                cancel: CancellationToken::new(),
            });
            let ingest = |stamp, source, retention| {
                let data = <Time as Message>::Codec::serialize(&Time::from_nanos(stamp)).unwrap();
                feed.state.lock().ingest(
                    Received {
                        message: data.into(),
                        source_time: Time::from_nanos(source),
                        transport_time: None,
                        sequence_number: stamp,
                        source_global_id: [0; 16].into(),
                    },
                    retention,
                );
            };
            ingest(10, 100, LiveRetention::default());
            ingest(20, 200, LiveRetention::default());
            let raw = feed.projection(&TimeProjection::SourceTime);
            let original = feed
                .state
                .lock()
                .indexes
                .get_mut(&TimeProjection::SourceTime)
                .unwrap()
                .snapshot();
            let projected = TimeProjection::field("", TimeEncoding::RosZTime).unwrap();
            let old = feed.projection(&projected);
            let cancelled = feed.state.lock().indexes[&projected].cancel.clone();
            ingest(
                30,
                300,
                LiveRetention {
                    samples: NonZeroUsize::MIN,
                    ..LiveRetention::default()
                },
            );
            drop(old);
            assert!(cancelled.is_cancelled());
            let replacement = feed.projection(&projected);
            let invalid = TimeProjection::field(".missing", TimeEncoding::RosZTime).unwrap();
            let bad = feed.projection(&invalid);
            let mut changes = feed.changed.subscribe();
            release.send(()).unwrap();
            blocker.await.unwrap();
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    changes.borrow_and_update();
                    let ready = {
                        let state = feed.state.lock();
                        !state.indexes[&TimeProjection::SourceTime].building
                            && !state.indexes[&projected].building
                    };
                    if ready {
                        break;
                    }
                    changes.changed().await.unwrap();
                }
            })
            .await
            .unwrap();
            let mut state = feed.state.lock();
            for (projection, time) in [(TimeProjection::SourceTime, 300), (projected, 30)] {
                let index = state.indexes.get_mut(&projection).unwrap();
                assert_eq!(index.records.len(), 1);
                let record = index.records.values().next().unwrap();
                assert_eq!(record.id, 2, "backfill must not revive evicted records");
                assert_eq!(record.time, Time::from_nanos(time));
                assert!(index.error.is_none());
                let snapshot = index.snapshot();
                assert!(Arc::ptr_eq(&snapshot, &index.snapshot()));
                assert!(original.is_empty(), "published snapshots stay immutable");
                assert!(!Arc::ptr_eq(&original, &snapshot));
                index.remove(2);
                assert!(index.snapshot().is_empty());
                assert_eq!(snapshot.len(), 1);
            }
            assert!(state.indexes[&invalid].error.is_some());
            drop(state);
            drop((raw, replacement, bad));
        });
    }
}
