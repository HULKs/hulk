use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use parking_lot::Mutex as ParkingMutex;
use tracing::debug;
use zenoh::{Session, sample::Sample};

use super::metadata::{self, PublicationId};
use crate::Result;
use crate::attachment::{Attachment, EndpointGlobalId};
use crate::entity::EndpointEntity;
use crate::graph::Graph;
use ros_z_protocol::qos::{QosDurability, QosHistory};

#[derive(Clone)]
pub(super) struct RetainedSample {
    pub(super) payload: zenoh::bytes::ZBytes,
    pub(super) encoding: Option<zenoh::bytes::Encoding>,
    pub(super) attachment: Attachment,
}

pub(super) struct TransientLocalCache {
    capacity: usize,
    samples: ParkingMutex<VecDeque<RetainedSample>>,
}

#[derive(Clone)]
struct OrderedReplaySample {
    sample: Sample,
    insertion_order: usize,
    publication_id: Option<PublicationId>,
}

pub(super) struct TransientLocalReplayGuard {
    cancelled: Arc<AtomicBool>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl TransientLocalReplayGuard {
    pub(super) fn new(cancelled: Arc<AtomicBool>, task: tokio::task::JoinHandle<()>) -> Self {
        Self {
            cancelled,
            task: Some(task),
        }
    }
}

impl Drop for TransientLocalReplayGuard {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

struct ReplayWindow {
    pending: bool,
    draining: bool,
    replay: Vec<OrderedReplaySample>,
    live: VecDeque<Sample>,
    replay_capacity: usize,
    live_capacity: usize,
    delivered_replay_ids: HashSet<PublicationId>,
}

impl ReplayWindow {
    fn new(live_capacity: usize) -> Self {
        Self {
            pending: true,
            draining: false,
            replay: Vec::with_capacity(live_capacity.min(1024)),
            live: VecDeque::with_capacity(live_capacity.min(1024)),
            replay_capacity: live_capacity,
            live_capacity,
            delivered_replay_ids: HashSet::new(),
        }
    }

    fn push_replay(&mut self, sample: OrderedReplaySample) {
        if self.replay_capacity == 0 {
            return;
        }
        if self.replay.len() >= self.replay_capacity {
            self.replay.remove(0);
        }
        self.replay.push(sample);
    }

    fn push_live(&mut self, sample: Sample) {
        if self.live_capacity == 0 {
            return;
        }
        if self.live.len() >= self.live_capacity {
            self.live.pop_front();
        }
        self.live.push_back(sample);
    }
}

struct SourceReplayState {
    last_delivered: Option<i64>,
    pending_replay_queries: usize,
    pending_samples: BTreeMap<i64, Sample>,
    capacity: usize,
}

impl SourceReplayState {
    fn new(capacity: usize) -> Self {
        Self {
            last_delivered: None,
            pending_replay_queries: 0,
            pending_samples: BTreeMap::new(),
            capacity,
        }
    }

    fn begin_replay_query(&mut self) {
        self.pending_replay_queries = self.pending_replay_queries.saturating_add(1);
    }

    fn begin_replay_query_if_idle(&mut self) {
        if self.pending_replay_queries == 0 {
            self.begin_replay_query();
        }
    }

    fn ingest(&mut self, sequence_number: i64, sample: Sample) -> Vec<Sample> {
        if self
            .last_delivered
            .is_some_and(|last_delivered| sequence_number <= last_delivered)
        {
            return Vec::new();
        }

        if self.pending_replay_queries > 0 {
            self.stage(sequence_number, sample);
            return Vec::new();
        }

        self.stage(sequence_number, sample);
        self.flush_ready()
    }

    fn finish_replay_query(&mut self) -> Vec<Sample> {
        self.pending_replay_queries = self.pending_replay_queries.saturating_sub(1);
        if self.pending_replay_queries == 0 {
            self.flush_ready()
        } else {
            Vec::new()
        }
    }

    fn stage(&mut self, sequence_number: i64, sample: Sample) {
        self.pending_samples
            .entry(sequence_number)
            .or_insert(sample);
        while self.pending_samples.len() > self.capacity {
            self.pending_samples.pop_first();
        }
    }

    fn flush_ready(&mut self) -> Vec<Sample> {
        let mut delivered = Vec::new();
        loop {
            let Some((&next_sequence_number, _)) = self.pending_samples.first_key_value() else {
                return delivered;
            };

            if self
                .last_delivered
                .is_some_and(|last_delivered| next_sequence_number <= last_delivered)
            {
                self.pending_samples.pop_first();
                continue;
            }

            let (sequence_number, sample) = self
                .pending_samples
                .pop_first()
                .expect("first_key_value confirmed a pending sample");
            self.last_delivered = Some(sequence_number);
            delivered.push(sample);
        }
    }
}

pub(super) struct TransientLocalReplayCoordinator {
    initial_window: ParkingMutex<ReplayWindow>,
    late_windows: ParkingMutex<HashMap<EndpointGlobalId, ReplayWindow>>,
    sources: ParkingMutex<HashMap<EndpointGlobalId, SourceReplayState>>,
    unknown_late_live: ParkingMutex<VecDeque<Sample>>,
    unknown_late_live_capacity: usize,
    handler: Arc<dyn Fn(Sample) + Send + Sync>,
    cancelled: Arc<AtomicBool>,
}

impl TransientLocalReplayCoordinator {
    #[cfg(test)]
    fn new_for_test(live_capacity: usize, handler: Arc<dyn Fn(Sample) + Send + Sync>) -> Self {
        Self::new(live_capacity, handler, Arc::new(AtomicBool::new(false)))
    }

    pub(super) fn new(
        live_capacity: usize,
        handler: Arc<dyn Fn(Sample) + Send + Sync>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            initial_window: ParkingMutex::new(ReplayWindow::new(live_capacity)),
            late_windows: ParkingMutex::new(HashMap::new()),
            sources: ParkingMutex::new(HashMap::new()),
            unknown_late_live: ParkingMutex::new(VecDeque::with_capacity(live_capacity.min(1024))),
            unknown_late_live_capacity: live_capacity,
            handler,
            cancelled,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn deliver(&self, sample: Sample) {
        if !self.is_cancelled() {
            (self.handler)(sample);
        }
    }

    fn source_state_mut(
        sources: &mut HashMap<EndpointGlobalId, SourceReplayState>,
        source_global_id: EndpointGlobalId,
        capacity: usize,
    ) -> &mut SourceReplayState {
        sources
            .entry(source_global_id)
            .or_insert_with(|| SourceReplayState::new(capacity))
    }

    fn deliver_sequenced(&self, samples: Vec<Sample>) {
        for sample in samples {
            self.deliver(sample);
        }
    }

    fn handle_sequenced_sample(
        &self,
        publication_id: PublicationId,
        sample: Sample,
        capacity: usize,
    ) {
        let source_global_id = publication_id.endpoint_global_id();
        let sequence_number = publication_id.sequence_number();
        let samples = {
            let mut sources = self.sources.lock();
            let state = Self::source_state_mut(&mut sources, source_global_id, capacity);
            state.ingest(sequence_number, sample)
        };
        self.deliver_sequenced(samples);
    }

    fn stage_pending_source_sample(
        &self,
        publication_id: PublicationId,
        sample: Sample,
        missing_capacity: Option<usize>,
    ) -> Option<Sample> {
        let source_global_id = publication_id.endpoint_global_id();
        let sequence_number = publication_id.sequence_number();
        let mut sources = self.sources.lock();
        let state = match sources.entry(source_global_id) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let Some(capacity) = missing_capacity else {
                    return Some(sample);
                };
                let mut state = SourceReplayState::new(capacity);
                state.begin_replay_query();
                entry.insert(state)
            }
        };
        if state.pending_replay_queries == 0 {
            return Some(sample);
        }
        let delivered = state.ingest(sequence_number, sample);
        debug_assert!(
            delivered.is_empty(),
            "pending replay ingestion should only stage samples"
        );
        None
    }

    fn handle_pending_source_sample(
        &self,
        publication_id: PublicationId,
        sample: Sample,
    ) -> Option<Sample> {
        self.stage_pending_source_sample(publication_id, sample, None)
    }

    fn finish_source_replay(&self, publisher_global_id: EndpointGlobalId) {
        let samples = {
            let mut sources = self.sources.lock();
            let Some(state) = sources.get_mut(&publisher_global_id) else {
                return;
            };
            state.finish_replay_query()
        };
        self.deliver_sequenced(samples);
    }

    pub(super) fn handle_live(self: &Arc<Self>, sample: Sample) {
        if self.is_cancelled() {
            return;
        }
        let mut sample = sample;
        let publication_id = metadata::publication_id_from_sample(&sample);
        if let Some(publication_id) = publication_id {
            let source_global_id = publication_id.endpoint_global_id();
            let mut late_windows = self.late_windows.lock();
            if let Some(window) = late_windows.get_mut(&source_global_id) {
                if window.pending {
                    match self.stage_pending_source_sample(
                        publication_id,
                        sample,
                        Some(window.live_capacity),
                    ) {
                        None => return,
                        Some(returned_sample) => {
                            window.push_live(returned_sample);
                            return;
                        }
                    }
                }
                if window.draining {
                    window.push_live(sample);
                    return;
                }
            }
        }

        if let Some(publication_id) = publication_id {
            match self.handle_pending_source_sample(publication_id, sample) {
                None => return,
                Some(returned_sample) => sample = returned_sample,
            }
        } else {
            match self.buffer_unknown_late_live_if_active(sample) {
                None => return,
                Some(returned_sample) => sample = returned_sample,
            }
        }

        let mut initial_window = self.initial_window.lock();
        if initial_window.pending || initial_window.draining {
            initial_window.push_live(sample);
            return;
        }
        if publication_id.is_some_and(|id| initial_window.delivered_replay_ids.contains(&id)) {
            return;
        }
        drop(initial_window);
        if let Some(publication_id) = publication_id {
            self.handle_sequenced_sample(publication_id, sample, self.unknown_late_live_capacity);
            return;
        }
        self.deliver(sample);
    }

    pub(super) fn begin_initial_publisher(
        &self,
        publisher_global_id: EndpointGlobalId,
        live_capacity: usize,
    ) {
        let mut sources = self.sources.lock();
        Self::source_state_mut(&mut sources, publisher_global_id, live_capacity)
            .begin_replay_query_if_idle();
    }

    pub(super) fn finish_initial_publisher(&self, publisher_global_id: EndpointGlobalId) {
        self.finish_source_replay(publisher_global_id);
    }

    fn begin_late_publisher(&self, publisher_global_id: EndpointGlobalId, live_capacity: usize) {
        let mut late_windows = self.late_windows.lock();
        late_windows
            .entry(publisher_global_id)
            .or_insert_with(|| ReplayWindow::new(live_capacity));
        let mut sources = self.sources.lock();
        Self::source_state_mut(&mut sources, publisher_global_id, live_capacity)
            .begin_replay_query_if_idle();
    }

    fn handle_late_replay(
        &self,
        publisher_global_id: EndpointGlobalId,
        sample: Sample,
        insertion_order: usize,
    ) {
        if self.is_cancelled() {
            return;
        }
        let publication_id = metadata::publication_id_from_sample(&sample);
        if publication_id.is_some_and(|id| {
            self.initial_window
                .lock()
                .delivered_replay_ids
                .contains(&id)
        }) {
            return;
        }
        if let Some(publication_id) = publication_id {
            self.handle_sequenced_sample(publication_id, sample, self.unknown_late_live_capacity);
            return;
        }

        let mut late_windows = self.late_windows.lock();
        let Some(window) = late_windows.get_mut(&publisher_global_id) else {
            drop(late_windows);
            self.deliver(sample);
            return;
        };
        if !window.pending {
            drop(late_windows);
            self.deliver(sample);
            return;
        }
        window.push_replay(OrderedReplaySample {
            sample,
            insertion_order,
            publication_id: None,
        });
    }

    fn finish_late_replay(self: &Arc<Self>, publisher_global_id: EndpointGlobalId) {
        if self.is_cancelled() {
            self.late_windows.lock().remove(&publisher_global_id);
            return;
        }
        let (samples, live) = {
            let mut late_windows = self.late_windows.lock();
            let Some(window) = late_windows.get_mut(&publisher_global_id) else {
                return;
            };
            window.draining = true;
            if replay_has_complete_publication_ids(
                window.replay.iter().map(|sample| sample.publication_id),
            ) {
                window
                    .replay
                    .sort_by_key(|sample| sample.publication_id.expect("checked above"));
            } else {
                window.replay.sort_by_key(|sample| sample.insertion_order);
            }
            (
                std::mem::take(&mut window.replay),
                std::mem::take(&mut window.live),
            )
        };

        let mut delivered = self.initial_window.lock().delivered_replay_ids.clone();
        for sample in samples {
            if should_deliver_replay_id(&mut delivered, sample.publication_id) {
                self.deliver(sample.sample);
            }
        }
        self.initial_window.lock().delivered_replay_ids = delivered.clone();

        self.finish_source_replay(publisher_global_id);
        self.drain_late_live(publisher_global_id, live, &delivered);
    }

    fn buffer_unknown_late_live_if_active(&self, sample: Sample) -> Option<Sample> {
        if !self
            .late_windows
            .lock()
            .values()
            .any(|window| window.pending || window.draining)
        {
            return Some(sample);
        }
        let mut unknown_late_live = self.unknown_late_live.lock();
        push_bounded_sample(
            &mut unknown_late_live,
            sample,
            self.unknown_late_live_capacity,
        );
        None
    }

    fn handle_replay(&self, sample: Sample, insertion_order: usize) {
        if self.is_cancelled() {
            return;
        }
        let publication_id = metadata::publication_id_from_sample(&sample);
        if let Some(publication_id) = publication_id {
            self.handle_sequenced_sample(publication_id, sample, self.unknown_late_live_capacity);
            return;
        }
        let mut initial_window = self.initial_window.lock();
        if !initial_window.pending {
            drop(initial_window);
            self.deliver(sample);
            return;
        }
        initial_window.push_replay(OrderedReplaySample {
            sample,
            insertion_order,
            publication_id,
        });
    }

    pub(super) fn finish_initial_replay(self: &Arc<Self>) {
        let (replay, live) = {
            let mut initial_window = self.initial_window.lock();
            initial_window.draining = true;
            if replay_has_complete_publication_ids(
                initial_window
                    .replay
                    .iter()
                    .map(|sample| sample.publication_id),
            ) {
                initial_window
                    .replay
                    .sort_by_key(|sample| sample.publication_id.expect("checked above"));
            } else {
                initial_window
                    .replay
                    .sort_by_key(|sample| sample.insertion_order);
            }
            let replay = std::mem::take(&mut initial_window.replay);
            let live = std::mem::take(&mut initial_window.live);
            (replay, live)
        };

        let mut delivered = HashSet::new();
        for sample in replay {
            if should_deliver_replay_id(&mut delivered, sample.publication_id) {
                self.deliver(sample.sample);
            }
        }
        self.initial_window.lock().delivered_replay_ids = delivered.clone();

        self.drain_live(live, &delivered);
    }

    fn drain_live(&self, mut live: VecDeque<Sample>, delivered: &HashSet<PublicationId>) {
        loop {
            for sample in live {
                if is_replay_duplicate(&sample, delivered) {
                    continue;
                }
                if let Some(publication_id) = metadata::publication_id_from_sample(&sample) {
                    self.handle_sequenced_sample(
                        publication_id,
                        sample,
                        self.unknown_late_live_capacity,
                    );
                    continue;
                }
                self.deliver(sample);
            }

            let mut initial_window = self.initial_window.lock();
            live = std::mem::take(&mut initial_window.live);
            if live.is_empty() {
                initial_window.pending = false;
                initial_window.draining = false;
                return;
            }
        }
    }

    fn drain_late_live(
        &self,
        publisher_global_id: EndpointGlobalId,
        mut live: VecDeque<Sample>,
        delivered: &HashSet<PublicationId>,
    ) {
        loop {
            for sample in live {
                if is_replay_duplicate(&sample, delivered) {
                    continue;
                }
                if let Some(publication_id) = metadata::publication_id_from_sample(&sample) {
                    self.handle_sequenced_sample(
                        publication_id,
                        sample,
                        self.unknown_late_live_capacity,
                    );
                    continue;
                }
                self.deliver(sample);
            }

            let mut late_windows = self.late_windows.lock();
            let Some(window) = late_windows.get_mut(&publisher_global_id) else {
                return;
            };
            live = std::mem::take(&mut window.live);
            if live.is_empty() {
                late_windows.remove(&publisher_global_id);
                let should_drain_unknown = !late_windows
                    .values()
                    .any(|window| window.pending || window.draining);
                drop(late_windows);
                if should_drain_unknown {
                    self.drain_unknown_late_live(delivered);
                }
                return;
            }
        }
    }

    fn drain_unknown_late_live(&self, delivered: &HashSet<PublicationId>) {
        loop {
            let unknown = std::mem::take(&mut *self.unknown_late_live.lock());
            if unknown.is_empty() {
                return;
            }
            for sample in unknown {
                if !is_replay_duplicate(&sample, delivered) {
                    self.deliver(sample);
                }
            }
        }
    }
}

fn push_bounded_sample(samples: &mut VecDeque<Sample>, sample: Sample, capacity: usize) {
    if capacity == 0 {
        return;
    }
    if samples.len() >= capacity {
        samples.pop_front();
    }
    samples.push_back(sample);
}

fn should_deliver_replay_id(
    delivered: &mut std::collections::HashSet<PublicationId>,
    publication_id: Option<PublicationId>,
) -> bool {
    match publication_id {
        Some(id) => delivered.insert(id),
        None => true,
    }
}

fn is_replay_duplicate(sample: &Sample, delivered: &HashSet<PublicationId>) -> bool {
    metadata::publication_id_from_sample(sample).is_some_and(|id| delivered.contains(&id))
}

fn replay_has_complete_publication_ids(
    publication_ids: impl IntoIterator<Item = Option<PublicationId>>,
) -> bool {
    publication_ids.into_iter().all(|id| id.is_some())
}

impl TransientLocalCache {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            samples: ParkingMutex::new(VecDeque::with_capacity(capacity)),
        }
    }

    pub(super) fn retain(&self, sample: RetainedSample) {
        let mut samples = self.samples.lock();
        if samples.len() >= self.capacity {
            samples.pop_front();
        }
        samples.push_back(sample);
    }

    pub(super) fn samples(&self) -> Vec<RetainedSample> {
        self.samples.lock().iter().cloned().collect()
    }
}

fn format_endpoint_global_id_hex(endpoint_global_id: EndpointGlobalId) -> String {
    endpoint_global_id
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn transient_local_replay_key(
    topic_key_expr: impl std::fmt::Display,
    publisher_global_id: EndpointGlobalId,
) -> String {
    format!(
        "{topic_key_expr}/__ros_z_transient_local/{}",
        format_endpoint_global_id_hex(publisher_global_id)
    )
}

pub(crate) fn transient_local_cache_capacity(
    qos: &ros_z_protocol::qos::QosProfile,
) -> Option<usize> {
    match qos.durability {
        QosDurability::TransientLocal => match qos.history {
            QosHistory::KeepLast(depth) => Some(depth),
            QosHistory::KeepAll => None,
        },
        QosDurability::Volatile => None,
    }
}

pub(crate) fn transient_local_replay_live_capacity(
    qos: &ros_z_protocol::qos::QosProfile,
) -> Option<usize> {
    match qos.history {
        QosHistory::KeepLast(depth) => Some(depth),
        QosHistory::KeepAll => None,
    }
}

async fn query_transient_local_replay(
    session: &Session,
    topic_key_expr: &str,
    publisher_global_id: EndpointGlobalId,
    timeout: Duration,
    coordinator: Arc<TransientLocalReplayCoordinator>,
) -> usize {
    if coordinator.is_cancelled() {
        return 0;
    }
    let replay_key = transient_local_replay_key(topic_key_expr, publisher_global_id);
    let expected_topic_key_expr = topic_key_expr.to_owned();
    let replay_order = Arc::new(AtomicUsize::new(0));
    let replay_order_callback = replay_order.clone();
    let replay_coordinator = coordinator.clone();
    let replies = match session
        .get(replay_key)
        .target(zenoh::query::QueryTarget::AllComplete)
        .consolidation(zenoh::query::ConsolidationMode::None)
        .accept_replies(zenoh::query::ReplyKeyExpr::Any)
        .timeout(timeout)
        .await
    {
        Ok(replies) => replies,
        Err(err) => {
            debug!("[SUB] Failed to query transient local replay: {}", err);
            return if coordinator.is_cancelled() {
                0
            } else {
                replay_order.load(Ordering::Relaxed)
            };
        }
    };
    while let Ok(reply) = replies.recv_async().await {
        match reply.into_result() {
            Ok(sample) if sample.key_expr().as_str() == expected_topic_key_expr => {
                let insertion_order = replay_order_callback.fetch_add(1, Ordering::Relaxed);
                replay_coordinator.handle_late_replay(publisher_global_id, sample, insertion_order);
            }
            Ok(sample) => debug!(
                "[SUB] Dropping transient local replay sample with unexpected key: {}",
                sample.key_expr()
            ),
            Err(err) => debug!("[SUB] Transient local replay query error: {}", err),
        }
    }
    if coordinator.is_cancelled() {
        0
    } else {
        replay_order.load(Ordering::Relaxed)
    }
}

pub(crate) async fn query_initial_transient_local_replay_async(
    session: &Session,
    topic_key_expr: &str,
    publisher_global_id: EndpointGlobalId,
    timeout: Duration,
    coordinator: Arc<TransientLocalReplayCoordinator>,
) -> Result<usize> {
    if coordinator.is_cancelled() {
        return Ok(0);
    }
    let replay_key = transient_local_replay_key(topic_key_expr, publisher_global_id);
    let expected_topic_key_expr = topic_key_expr.to_owned();
    let replay_order = Arc::new(AtomicUsize::new(0));
    let replay_order_callback = replay_order.clone();
    let replay_coordinator = coordinator.clone();
    let replies = session
        .get(replay_key)
        .target(zenoh::query::QueryTarget::AllComplete)
        .consolidation(zenoh::query::ConsolidationMode::None)
        .accept_replies(zenoh::query::ReplyKeyExpr::Any)
        .timeout(timeout)
        .await
        .map_err(|source| crate::Error::zenoh("query transient-local replay", source))?;
    while let Ok(reply) = replies.recv_async().await {
        match reply.into_result() {
            Ok(sample) if sample.key_expr().as_str() == expected_topic_key_expr => {
                let insertion_order = replay_order_callback.fetch_add(1, Ordering::Relaxed);
                replay_coordinator.handle_replay(sample, insertion_order);
            }
            Ok(sample) => debug!(
                "[SUB] Dropping transient local replay sample with unexpected key: {}",
                sample.key_expr()
            ),
            Err(err) => debug!("[SUB] Transient local replay query error: {}", err),
        }
    }
    if coordinator.is_cancelled() {
        Ok(0)
    } else {
        Ok(replay_order.load(Ordering::Relaxed))
    }
}

fn replay_capable_publisher(endpoint: &EndpointEntity) -> Option<(EndpointGlobalId, usize)> {
    if !matches!(endpoint.qos.durability, QosDurability::TransientLocal) {
        return None;
    }
    let QosHistory::KeepLast(depth) = endpoint.qos.history else {
        return None;
    };
    Some((EndpointGlobalId::from(endpoint), depth))
}

pub(super) fn replay_capable_publishers(
    graph: &Graph,
    topic: &str,
) -> Vec<(EndpointGlobalId, usize)> {
    graph
        .view()
        .publishers_on(topic)
        .into_iter()
        .filter_map(|endpoint| replay_capable_publisher(&endpoint))
        .collect()
}

pub(super) fn initial_replay_plan(
    publishers: impl IntoIterator<Item = (EndpointGlobalId, usize)>,
) -> (Vec<(EndpointGlobalId, usize)>, HashSet<EndpointGlobalId>) {
    let publishers = publishers.into_iter().collect::<Vec<_>>();
    let seen = publishers
        .iter()
        .map(|(endpoint_global_id, _)| *endpoint_global_id)
        .collect();
    (publishers, seen)
}

fn begin_unseen_late_publishers(
    seen: &mut HashSet<EndpointGlobalId>,
    coordinator: &TransientLocalReplayCoordinator,
    publishers: impl IntoIterator<Item = (EndpointGlobalId, usize)>,
) -> Vec<EndpointGlobalId> {
    let mut discovered = Vec::new();
    for (publisher_global_id, depth) in publishers {
        if !seen.insert(publisher_global_id) {
            continue;
        }
        coordinator.begin_late_publisher(publisher_global_id, depth);
        discovered.push(publisher_global_id);
    }
    discovered
}

pub(crate) fn spawn_transient_local_replay_task(
    graph: Arc<Graph>,
    topic: String,
    coordinator: Arc<TransientLocalReplayCoordinator>,
    session: Session,
    topic_key_expr: String,
    timeout: Duration,
    initial_seen: HashSet<EndpointGlobalId>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut seen = initial_seen;
        let mut changes = graph.subscribe_changes();
        loop {
            if coordinator.is_cancelled() {
                return;
            }
            changes.mark_seen();
            let discovered = begin_unseen_late_publishers(
                &mut seen,
                &coordinator,
                replay_capable_publishers(&graph, &topic),
            );
            if coordinator.is_cancelled() {
                return;
            }
            for publisher_global_id in discovered {
                if coordinator.is_cancelled() {
                    return;
                }
                query_transient_local_replay(
                    &session,
                    &topic_key_expr,
                    publisher_global_id,
                    timeout,
                    coordinator.clone(),
                )
                .await;
                if coordinator.is_cancelled() {
                    return;
                }
                coordinator.finish_late_replay(publisher_global_id);
            }
            if changes.changed().await.is_none() {
                return;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_window_drops_duplicate_publication_ids() {
        let mut delivered = std::collections::HashSet::new();
        let id = PublicationId::new(EndpointGlobalId::from([1; 16]), 7);

        assert!(should_deliver_replay_id(&mut delivered, Some(id)));
        assert!(!should_deliver_replay_id(&mut delivered, Some(id)));
        assert!(should_deliver_replay_id(&mut delivered, None));
    }

    fn sample_with_payload(payload: &str) -> Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        zenoh::sample::SampleBuilder::put(key_expr, payload).into()
    }

    fn sample_with_publication_id(payload: &str, publication_id: PublicationId) -> Sample {
        let key_expr = "test/key".parse::<zenoh::key_expr::KeyExpr>().unwrap();
        let attachment = Attachment::new(
            publication_id.sequence_number(),
            publication_id.endpoint_global_id(),
        );
        zenoh::sample::SampleBuilder::put(key_expr, payload)
            .attachment(attachment)
            .into()
    }

    fn sample_payload(sample: &Sample) -> String {
        String::from_utf8(sample.payload().to_bytes().to_vec()).unwrap()
    }

    fn ingest_payload(
        state: &mut SourceReplayState,
        sequence_number: i64,
        payload: &str,
    ) -> Vec<String> {
        state
            .ingest(sequence_number, sample_with_payload(payload))
            .into_iter()
            .map(|sample| sample_payload(&sample))
            .collect()
    }

    fn finish_payloads(state: &mut SourceReplayState) -> Vec<String> {
        state
            .finish_replay_query()
            .into_iter()
            .map(|sample| sample_payload(&sample))
            .collect()
    }

    #[test]
    fn source_replay_state_stages_samples_while_replay_is_pending() {
        let mut state = SourceReplayState::new(3);
        state.begin_replay_query();

        assert!(ingest_payload(&mut state, 2, "live-2").is_empty());
        assert!(ingest_payload(&mut state, 1, "replay-1").is_empty());

        assert_eq!(finish_payloads(&mut state), ["replay-1", "live-2"]);
    }

    #[test]
    fn source_replay_state_drops_duplicate_sequence_numbers() {
        let mut state = SourceReplayState::new(3);
        state.begin_replay_query();

        assert!(ingest_payload(&mut state, 1, "live-1").is_empty());
        assert!(ingest_payload(&mut state, 1, "replay-1-duplicate").is_empty());

        assert_eq!(finish_payloads(&mut state), ["live-1"]);
        assert!(ingest_payload(&mut state, 1, "late-duplicate").is_empty());
    }

    #[test]
    fn source_replay_state_bounds_pending_samples_by_capacity() {
        let mut state = SourceReplayState::new(2);
        state.begin_replay_query();

        assert!(ingest_payload(&mut state, 1, "one").is_empty());
        assert!(ingest_payload(&mut state, 2, "two").is_empty());
        assert!(ingest_payload(&mut state, 3, "three").is_empty());

        assert_eq!(finish_payloads(&mut state), ["two", "three"]);
    }

    #[test]
    fn source_replay_state_starts_finished_replay_at_lowest_retained_sequence() {
        let mut state = SourceReplayState::new(2);
        state.begin_replay_query();

        assert!(ingest_payload(&mut state, 9, "nine").is_empty());
        assert!(ingest_payload(&mut state, 10, "ten").is_empty());

        assert_eq!(finish_payloads(&mut state), ["nine", "ten"]);
    }

    #[test]
    fn source_replay_state_ignores_double_replay_completion() {
        let mut state = SourceReplayState::new(2);
        state.begin_replay_query();

        assert!(ingest_payload(&mut state, 1, "one").is_empty());
        assert_eq!(finish_payloads(&mut state), ["one"]);
        assert!(finish_payloads(&mut state).is_empty());
    }

    #[test]
    fn replay_window_drops_oldest_live_sample_at_capacity() {
        let mut window = ReplayWindow::new(2);

        window.push_live(sample_with_payload("live-1"));
        window.push_live(sample_with_payload("live-2"));
        window.push_live(sample_with_payload("live-3"));

        let payloads = window.live.iter().map(sample_payload).collect::<Vec<_>>();
        assert_eq!(payloads, ["live-2", "live-3"]);
    }

    #[test]
    fn replay_window_drops_oldest_replay_sample_at_capacity() {
        let mut window = ReplayWindow::new(2);

        for (insertion_order, payload) in
            ["replay-1", "replay-2", "replay-3"].into_iter().enumerate()
        {
            window.push_replay(OrderedReplaySample {
                sample: sample_with_payload(payload),
                insertion_order,
                publication_id: None,
            });
        }

        let payloads = window
            .replay
            .iter()
            .map(|sample| sample_payload(&sample.sample))
            .collect::<Vec<_>>();
        assert_eq!(payloads, ["replay-2", "replay-3"]);
    }

    #[test]
    fn initial_replay_plan_seeds_seen_with_only_queried_publishers() {
        let first_publisher_global_id = EndpointGlobalId::from([1_u8; 16]);
        let second_publisher_global_id = EndpointGlobalId::from([2_u8; 16]);

        let (publishers, seen) = initial_replay_plan([
            (first_publisher_global_id, 1),
            (second_publisher_global_id, 2),
        ]);

        assert_eq!(
            publishers,
            [
                (first_publisher_global_id, 1),
                (second_publisher_global_id, 2)
            ]
        );
        assert_eq!(
            seen,
            HashSet::from([first_publisher_global_id, second_publisher_global_id])
        );
    }

    #[test]
    fn replay_coordinator_bounds_initial_replay_staging() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(2, handler));

        coordinator.handle_replay(sample_with_payload("replay-1"), 0);
        coordinator.handle_replay(sample_with_payload("replay-2"), 1);
        coordinator.handle_replay(sample_with_payload("replay-3"), 2);
        coordinator.finish_initial_replay();

        assert_eq!(*received.lock(), ["replay-2", "replay-3"]);
    }

    #[test]
    fn replay_coordinator_flushes_initial_metadata_replay_before_same_source_live() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([14_u8; 16]);

        coordinator.begin_initial_publisher(publisher_global_id, 3);
        coordinator.handle_live(sample_with_publication_id(
            "live-2",
            PublicationId::new(publisher_global_id, 2),
        ));
        coordinator.handle_replay(
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );

        assert!(received.lock().is_empty());

        coordinator.finish_initial_publisher(publisher_global_id);

        assert_eq!(*received.lock(), ["replay-1", "live-2"]);
    }

    #[test]
    fn replay_coordinator_keeps_later_initial_source_live_pending_until_its_query_finishes() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let first_publisher_global_id = EndpointGlobalId::from([15_u8; 16]);
        let second_publisher_global_id = EndpointGlobalId::from([16_u8; 16]);

        coordinator.begin_initial_publisher(first_publisher_global_id, 3);
        coordinator.begin_initial_publisher(second_publisher_global_id, 3);
        coordinator.handle_live(sample_with_publication_id(
            "second-live-2",
            PublicationId::new(second_publisher_global_id, 2),
        ));
        coordinator.handle_replay(
            sample_with_publication_id(
                "first-replay-1",
                PublicationId::new(first_publisher_global_id, 1),
            ),
            0,
        );
        coordinator.finish_initial_publisher(first_publisher_global_id);

        assert_eq!(*received.lock(), ["first-replay-1"]);

        coordinator.handle_replay(
            sample_with_publication_id(
                "second-replay-1",
                PublicationId::new(second_publisher_global_id, 1),
            ),
            1,
        );
        coordinator.finish_initial_publisher(second_publisher_global_id);

        assert_eq!(
            *received.lock(),
            ["first-replay-1", "second-replay-1", "second-live-2"]
        );
    }

    #[test]
    fn replay_coordinator_bounds_late_replay_staging() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([3_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 2);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("replay-1"), 0);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("replay-2"), 1);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("replay-3"), 2);
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["replay-2", "replay-3"]);
    }

    #[test]
    fn replay_coordinator_buffers_live_samples_arriving_during_replay_drain() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let coordinator_slot = Arc::new(ParkingMutex::new(None));

        let handler_received = received.clone();
        let handler_coordinator = coordinator_slot.clone();
        let handler = Arc::new(move |sample: Sample| {
            let payload = sample_payload(&sample);
            handler_received.lock().push(payload.clone());
            if payload == "cached-1" {
                let coordinator: Arc<TransientLocalReplayCoordinator> = handler_coordinator
                    .lock()
                    .as_ref()
                    .cloned()
                    .expect("coordinator should be installed before replay");
                coordinator.handle_live(sample_with_payload("live-during"));
            }
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(10, handler));
        *coordinator_slot.lock() = Some(coordinator.clone());

        coordinator.handle_replay(sample_with_payload("cached-1"), 0);
        coordinator.handle_replay(sample_with_payload("cached-2"), 1);
        coordinator.handle_replay(sample_with_payload("cached-3"), 2);
        coordinator.handle_live(sample_with_payload("live-before-drain"));

        coordinator.finish_initial_replay();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while received.lock().len() < 5 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }

        assert_eq!(
            *received.lock(),
            [
                "cached-1",
                "cached-2",
                "cached-3",
                "live-before-drain",
                "live-during",
            ]
        );
    }

    #[test]
    fn replay_coordinator_buffers_metadata_live_arriving_during_initial_replay_drain() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let coordinator_slot = Arc::new(ParkingMutex::new(None));
        let publisher_global_id = EndpointGlobalId::from([17_u8; 16]);

        let handler_received = received.clone();
        let handler_coordinator = coordinator_slot.clone();
        let handler = Arc::new(move |sample: Sample| {
            let payload = sample_payload(&sample);
            handler_received.lock().push(payload.clone());
            if payload == "fallback-replay-1" {
                let coordinator: Arc<TransientLocalReplayCoordinator> = handler_coordinator
                    .lock()
                    .as_ref()
                    .cloned()
                    .expect("coordinator should be installed before replay");
                coordinator.handle_live(sample_with_publication_id(
                    "metadata-live-during-drain",
                    PublicationId::new(publisher_global_id, 2),
                ));
            }
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(10, handler));
        *coordinator_slot.lock() = Some(coordinator.clone());

        coordinator.begin_initial_publisher(publisher_global_id, 3);
        coordinator.handle_replay(
            sample_with_publication_id(
                "source-replay-1",
                PublicationId::new(publisher_global_id, 1),
            ),
            0,
        );
        coordinator.finish_initial_publisher(publisher_global_id);
        coordinator.handle_replay(sample_with_payload("fallback-replay-1"), 1);
        coordinator.handle_replay(sample_with_payload("fallback-replay-2"), 2);

        coordinator.finish_initial_replay();

        assert_eq!(
            *received.lock(),
            [
                "source-replay-1",
                "fallback-replay-1",
                "fallback-replay-2",
                "metadata-live-during-drain",
            ]
        );
    }

    #[test]
    fn replay_coordinator_suppresses_duplicate_replay_ids_during_initial_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publication_id = PublicationId::new(EndpointGlobalId::from([3_u8; 16]), 7);

        coordinator.handle_replay(sample_with_publication_id("replay", publication_id), 0);
        coordinator.handle_replay(
            sample_with_publication_id("duplicate-replay", publication_id),
            1,
        );
        coordinator.finish_initial_replay();

        assert_eq!(*received.lock(), ["replay"]);
    }

    #[test]
    fn replay_coordinator_delivers_duplicate_payloads_without_publication_ids() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([4_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("same"), 0);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("same"), 1);
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["same", "same"]);
    }

    #[test]
    fn seen_initial_replay_publisher_is_not_queried_again_as_late_publisher() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = TransientLocalReplayCoordinator::new_for_test(1, handler);
        let publisher_global_id = EndpointGlobalId::from([9_u8; 16]);
        let mut seen = HashSet::from([publisher_global_id]);

        let discovered =
            begin_unseen_late_publishers(&mut seen, &coordinator, [(publisher_global_id, 1)]);

        assert!(discovered.is_empty());
        assert!(coordinator.late_windows.lock().is_empty());
    }

    #[test]
    fn no_id_samples_are_not_payload_deduped_by_late_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(1, handler));
        let publisher_global_id = EndpointGlobalId::from([8_u8; 16]);

        coordinator.handle_replay(sample_with_payload("same"), 0);
        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 1);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("same"), 0);
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["same", "same"]);
    }

    #[test]
    fn replay_coordinator_buffers_unknown_source_live_during_late_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([9_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("late-replay"), 0);
        coordinator.handle_live(sample_with_payload("unknown-live"));

        assert!(received.lock().is_empty());

        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["late-replay", "unknown-live"]);
    }

    #[test]
    fn replay_coordinator_bounds_unknown_source_late_live() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(2, handler));
        let publisher_global_id = EndpointGlobalId::from([10_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 2);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("late-replay"), 0);
        coordinator.handle_live(sample_with_payload("unknown-1"));
        coordinator.handle_live(sample_with_payload("unknown-2"));
        coordinator.handle_live(sample_with_payload("unknown-3"));
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["late-replay", "unknown-2", "unknown-3"]);
    }

    #[test]
    fn replay_coordinator_passes_unknown_source_live_when_no_late_window_active() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(2, handler));

        coordinator.finish_initial_replay();
        coordinator.handle_live(sample_with_payload("unknown-live"));

        assert_eq!(*received.lock(), ["unknown-live"]);
    }

    #[test]
    fn replay_coordinator_keeps_independent_source_ordering() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let first = EndpointGlobalId::from([21_u8; 16]);
        let second = EndpointGlobalId::from([22_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(first, 3);
        coordinator.begin_late_publisher(second, 3);
        coordinator.handle_live(sample_with_publication_id(
            "first-2",
            PublicationId::new(first, 2),
        ));
        coordinator.handle_live(sample_with_publication_id(
            "second-2",
            PublicationId::new(second, 2),
        ));
        coordinator.handle_late_replay(
            first,
            sample_with_publication_id("first-1", PublicationId::new(first, 1)),
            0,
        );
        coordinator.handle_late_replay(
            second,
            sample_with_publication_id("second-1", PublicationId::new(second, 1)),
            0,
        );
        coordinator.finish_late_replay(first);
        coordinator.finish_late_replay(second);

        assert_eq!(
            *received.lock(),
            ["first-1", "first-2", "second-1", "second-2"]
        );
    }

    #[test]
    fn replay_coordinator_keeps_late_window_active_while_draining() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let coordinator_slot = Arc::new(ParkingMutex::new(None));
        let publisher_global_id = EndpointGlobalId::from([5_u8; 16]);

        let handler_received = received.clone();
        let handler_coordinator = coordinator_slot.clone();
        let handler = Arc::new(move |sample: Sample| {
            let payload = sample_payload(&sample);
            handler_received.lock().push(payload.clone());
            if payload == "late-replay-1" {
                let coordinator: Arc<TransientLocalReplayCoordinator> = handler_coordinator
                    .lock()
                    .as_ref()
                    .cloned()
                    .expect("coordinator should be installed before replay");
                coordinator.handle_live(sample_with_publication_id(
                    "late-live-during-drain",
                    PublicationId::new(publisher_global_id, 4),
                ));
            }
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        *coordinator_slot.lock() = Some(coordinator.clone());

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("late-replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("late-replay-2", PublicationId::new(publisher_global_id, 2)),
            1,
        );
        coordinator.handle_live(sample_with_publication_id(
            "late-live-before-drain",
            PublicationId::new(publisher_global_id, 3),
        ));

        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(
            *received.lock(),
            [
                "late-replay-1",
                "late-replay-2",
                "late-live-before-drain",
                "late-live-during-drain",
            ]
        );
    }

    #[test]
    fn replay_coordinator_suppresses_duplicate_publication_ids_during_late_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([6_u8; 16]);
        let publication_id = PublicationId::new(publisher_global_id, 7);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay", publication_id),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("duplicate-replay", publication_id),
            1,
        );
        coordinator.handle_live(sample_with_publication_id("duplicate-live", publication_id));
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["replay"]);
    }

    #[test]
    fn replay_coordinator_deduplicates_live_sample_against_later_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([11_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.handle_live(sample_with_publication_id(
            "live-1",
            PublicationId::new(publisher_global_id, 1),
        ));
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["live-1"]);
    }

    #[test]
    fn replay_coordinator_orders_late_replay_before_buffered_live() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([12_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_live(sample_with_publication_id(
            "live-3",
            PublicationId::new(publisher_global_id, 3),
        ));
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-2", PublicationId::new(publisher_global_id, 2)),
            1,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["replay-1", "replay-2", "live-3"]);
    }

    #[test]
    fn replay_coordinator_prefers_live_duplicate_during_pending_late_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([18_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_live(sample_with_publication_id(
            "live-2",
            PublicationId::new(publisher_global_id, 2),
        ));
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id(
                "replay-2-duplicate",
                PublicationId::new(publisher_global_id, 2),
            ),
            1,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["replay-1", "live-2"]);
    }

    #[test]
    fn replay_coordinator_sequences_pending_late_live_if_source_state_is_not_ready() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([20_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator
            .late_windows
            .lock()
            .insert(publisher_global_id, ReplayWindow::new(3));
        coordinator.handle_live(sample_with_publication_id(
            "live-2",
            PublicationId::new(publisher_global_id, 2),
        ));
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id(
                "replay-2-duplicate",
                PublicationId::new(publisher_global_id, 2),
            ),
            1,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["replay-1", "live-2"]);
    }

    #[test]
    fn replay_coordinator_orders_live_samples_during_pending_late_replay() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(4, handler));
        let publisher_global_id = EndpointGlobalId::from([19_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 4);
        coordinator.handle_live(sample_with_publication_id(
            "live-4",
            PublicationId::new(publisher_global_id, 4),
        ));
        coordinator.handle_live(sample_with_publication_id(
            "live-3",
            PublicationId::new(publisher_global_id, 3),
        ));
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-2", PublicationId::new(publisher_global_id, 2)),
            1,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(
            *received.lock(),
            ["replay-1", "replay-2", "live-3", "live-4"]
        );
    }

    #[test]
    fn replay_coordinator_preserves_monotonic_order_after_live_sample_escaped() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publisher_global_id = EndpointGlobalId::from([13_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.handle_live(sample_with_publication_id(
            "live-3",
            PublicationId::new(publisher_global_id, 3),
        ));
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-1", PublicationId::new(publisher_global_id, 1)),
            0,
        );
        coordinator.handle_late_replay(
            publisher_global_id,
            sample_with_publication_id("replay-2", PublicationId::new(publisher_global_id, 2)),
            1,
        );
        coordinator.finish_late_replay(publisher_global_id);

        assert_eq!(*received.lock(), ["live-3"]);
    }

    #[test]
    fn replay_coordinator_stops_delivery_after_cancellation() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let cancelled = Arc::new(AtomicBool::new(false));
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new(
            3,
            handler,
            cancelled.clone(),
        ));
        let publisher_global_id = EndpointGlobalId::from([7_u8; 16]);

        coordinator.finish_initial_replay();
        coordinator.begin_late_publisher(publisher_global_id, 3);
        coordinator.handle_late_replay(publisher_global_id, sample_with_payload("replay"), 0);
        cancelled.store(true, Ordering::Release);
        coordinator.finish_late_replay(publisher_global_id);
        coordinator.handle_live(sample_with_payload("live"));

        assert!(received.lock().is_empty());
    }

    #[test]
    fn cancelled_coordinator_does_not_deliver_live_samples() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let cancelled = Arc::new(AtomicBool::new(false));
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new(
            1,
            handler,
            cancelled.clone(),
        ));

        coordinator.finish_initial_replay();
        cancelled.store(true, Ordering::Release);
        coordinator.handle_live(sample_with_payload("live-after-cancel"));

        assert!(received.lock().is_empty());
    }

    #[test]
    fn replay_coordinator_opens_all_new_late_windows_before_queries() {
        let handler = Arc::new(|_sample: Sample| {});
        let coordinator = TransientLocalReplayCoordinator::new_for_test(3, handler);
        let mut seen = HashSet::new();
        let first_publisher_global_id = EndpointGlobalId::from([8_u8; 16]);
        let second_publisher_global_id = EndpointGlobalId::from([9_u8; 16]);

        let discovered = begin_unseen_late_publishers(
            &mut seen,
            &coordinator,
            [
                (first_publisher_global_id, 2),
                (second_publisher_global_id, 3),
            ],
        );

        assert_eq!(
            discovered,
            [first_publisher_global_id, second_publisher_global_id]
        );
        let late_windows = coordinator.late_windows.lock();
        assert!(late_windows.contains_key(&first_publisher_global_id));
        assert!(late_windows.contains_key(&second_publisher_global_id));
        drop(late_windows);
        let discovered = begin_unseen_late_publishers(
            &mut seen,
            &coordinator,
            [
                (first_publisher_global_id, 2),
                (second_publisher_global_id, 3),
            ],
        );
        assert!(discovered.is_empty());
    }

    #[test]
    fn replay_coordinator_suppresses_live_duplicate_after_replay_window() {
        let received = Arc::new(ParkingMutex::new(Vec::new()));
        let handler_received = received.clone();
        let handler = Arc::new(move |sample: Sample| {
            handler_received.lock().push(sample_payload(&sample));
        });
        let coordinator = Arc::new(TransientLocalReplayCoordinator::new_for_test(3, handler));
        let publication_id = PublicationId::new(EndpointGlobalId::from([2_u8; 16]), 7);

        coordinator.handle_replay(sample_with_publication_id("replay", publication_id), 0);
        coordinator.finish_initial_replay();
        coordinator.handle_live(sample_with_publication_id("duplicate-live", publication_id));
        coordinator.handle_live(sample_with_payload("live-without-id"));

        assert_eq!(*received.lock(), ["replay", "live-without-id"]);
    }
}
