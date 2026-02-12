use std::{
    collections::{HashMap, HashSet},
    future::Future,
    hash::{Hash, Hasher},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    cache::GlobalCache,
    error::{Error, Result},
    keyspace::{effective_namespace, from_nanos, source_key, to_nanos},
    storage::{DurableStats, Storage},
    types::{
        BackendStats, OpenMode, PlaneKind, SourceSpec, SourceStats, StreamRecord, TimelineBucket,
        TimelineSummary,
    },
};
use hulkz::{Node, Session, Timestamp};
use tokio::sync::{broadcast, mpsc, mpsc::error::TrySendError, oneshot, watch, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, warn};

const DEFAULT_CACHE_BUDGET_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_MAX_SEGMENT_BYTES: u64 = 256 * 1024 * 1024;
const DEFAULT_WRITE_QUEUE_CAPACITY: usize = 4096;
const INGEST_CAPACITY: usize = 64;
const LIVE_UPDATES_CAPACITY: usize = 1024;

pub struct StreamBackendBuilder {
    session: Session,
    open_mode: OpenMode,
    storage_path: Option<std::path::PathBuf>,
    cache_budget_bytes: usize,
    max_segment_bytes: u64,
    write_queue_capacity: usize,
}

impl StreamBackendBuilder {
    /// Creates a backend builder bound to an existing hulkz session.
    pub fn new(session: Session) -> Self {
        Self {
            session,
            open_mode: OpenMode::ReadWrite,
            storage_path: None,
            cache_budget_bytes: DEFAULT_CACHE_BUDGET_BYTES,
            max_segment_bytes: DEFAULT_MAX_SEGMENT_BYTES,
            write_queue_capacity: DEFAULT_WRITE_QUEUE_CAPACITY,
        }
    }

    /// Selects read-only vs read-write storage behavior.
    pub fn open_mode(mut self, mode: OpenMode) -> Self {
        self.open_mode = mode;
        self
    }

    /// Sets the root directory (or file in read-only mode) for durable storage.
    pub fn storage_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.storage_path = Some(path.into());
        self
    }

    /// Sets the shared in-memory cache budget in bytes.
    pub fn cache_budget_bytes(mut self, bytes: usize) -> Self {
        self.cache_budget_bytes = bytes;
        self
    }

    /// Sets the segment roll threshold for managed storage.
    pub fn max_segment_bytes(mut self, bytes: u64) -> Self {
        self.max_segment_bytes = bytes;
        self
    }

    /// Sets durable writer queue capacity. Policy is blocking backpressure when full.
    pub fn write_queue_capacity(mut self, capacity: usize) -> Self {
        self.write_queue_capacity = capacity.max(1);
        self
    }

    /// Builds backend handle and explicit driver future.
    pub async fn build(self) -> Result<(StreamBackend, StreamDriver)> {
        let storage_path = self
            .storage_path
            .unwrap_or_else(|| std::env::temp_dir().join("hulkz-stream"));
        info!(
            mode = ?self.open_mode,
            storage_path = %storage_path.display(),
            cache_budget_bytes = self.cache_budget_bytes,
            max_segment_bytes = self.max_segment_bytes,
            write_queue_capacity = self.write_queue_capacity,
            namespace = self.session.namespace(),
            "building stream backend",
        );
        let storage = Storage::open(self.open_mode, storage_path, self.max_segment_bytes).await?;

        let node = self
            .session
            .create_node("hulkz-stream")
            .build()
            .await
            .map_err(Error::from)?;

        let (control_tx, control_rx) = mpsc::unbounded_channel();
        let (write_tx, write_rx) = mpsc::channel::<WriteRequest>(self.write_queue_capacity);

        let (backend_stats_tx, _backend_stats_rx) = watch::channel(BackendStats::default());
        let (target_namespace_tx, target_namespace_rx) =
            watch::channel(self.session.namespace().to_string());
        let (ingest_enabled_tx, ingest_enabled_rx) = watch::channel(true);

        let inner = Arc::new(BackendInner {
            session: self.session,
            node,
            open_mode: self.open_mode,
            storage,
            cache: Arc::new(Mutex::new(GlobalCache::new(self.cache_budget_bytes))),
            control_tx,
            backend_stats_tx,
            active_sources: AtomicUsize::new(0),
            active_subscribers: AtomicUsize::new(0),
            target_namespace_tx,
            ingest_enabled_tx,
            is_closed: AtomicBool::new(false),
            writer_enqueued: AtomicU64::new(0),
            writer_dequeued: AtomicU64::new(0),
            writer_queue_high_watermark: AtomicUsize::new(0),
            writer_backpressure_events: AtomicU64::new(0),
        });

        let backend = StreamBackend {
            inner: inner.clone(),
        };

        let driver = StreamDriver::new(async move {
            run_driver(
                inner,
                control_rx,
                write_tx,
                write_rx,
                target_namespace_rx,
                ingest_enabled_rx,
            )
            .await
        });

        info!("stream backend build complete");
        Ok((backend, driver))
    }
}

#[derive(Clone)]
pub struct StreamBackend {
    inner: Arc<BackendInner>,
}

impl StreamBackend {
    /// Acquires or reuses a deduplicated source runtime and returns a query handle.
    pub async fn source(&self, spec: SourceSpec) -> Result<SourceHandle> {
        if spec.path.scope() == hulkz::Scope::Private && spec.node_override.is_none() {
            return Err(Error::NodeRequiredForPrivate);
        }
        let request_key = source_key(&spec);
        debug!(
            source_key = %request_key,
            plane = ?spec.plane,
            scope = %spec.path.scope().as_str(),
            path = %spec.path.path(),
            "acquiring source handle",
        );

        let (tx, rx) = oneshot::channel();
        self.inner
            .control_tx
            .send(Command::AcquireSource { spec, response: tx })
            .map_err(|_| Error::ControlChannelClosed)?;

        let acquired = rx.await.map_err(|_| Error::ResponseChannelClosed)??;

        let handle = SourceHandle {
            inner: self.inner.clone(),
            spec: acquired.spec,
            stats_rx: acquired.stats_rx,
            live_tx: acquired.live_tx,
            _lease: Arc::new(HandleLease {
                source_key: acquired.source_key,
                control_tx: self.inner.control_tx.clone(),
            }),
        };
        debug!(source_key = %request_key, "source handle acquired");
        Ok(handle)
    }

    /// Updates target namespace used by follow-target sources.
    pub async fn set_target_namespace(&self, namespace: impl Into<String>) -> Result<()> {
        let namespace = namespace.into();
        info!(%namespace, "updating target namespace");
        let (tx, rx) = oneshot::channel();
        self.inner
            .control_tx
            .send(Command::SetTargetNamespace {
                namespace,
                response: tx,
            })
            .map_err(|_| Error::ControlChannelClosed)?;
        rx.await.map_err(|_| Error::ResponseChannelClosed)?
    }

    /// Globally enables or pauses live ingest workers.
    pub async fn set_ingest_enabled(&self, enabled: bool) -> Result<()> {
        info!(enabled, "updating ingest enabled state");
        let (tx, rx) = oneshot::channel();
        self.inner
            .control_tx
            .send(Command::SetIngestEnabled {
                enabled,
                response: tx,
            })
            .map_err(|_| Error::ControlChannelClosed)?;
        rx.await.map_err(|_| Error::ResponseChannelClosed)?
    }

    /// Sets or clears the global scrub working-set window used by cache eviction.
    pub async fn set_scrub_window(&self, window: Option<(Timestamp, Timestamp)>) -> Result<()> {
        let mut cache = self.inner.cache.lock().await;
        cache.set_scrub_window(window);
        self.inner.publish_backend_stats(&cache).await;
        Ok(())
    }

    pub fn stats_snapshot(&self) -> BackendStats {
        self.inner.backend_stats_tx.borrow().clone()
    }

    pub fn stats_watch(&self) -> watch::Receiver<BackendStats> {
        self.inner.backend_stats_tx.subscribe()
    }

    /// Builds an aggregate timeline across all sources in the time range.
    pub async fn timeline_aggregate(
        &self,
        start: Timestamp,
        end: Timestamp,
        buckets: usize,
    ) -> Result<TimelineSummary> {
        if buckets == 0 {
            return Err(Error::InvalidBucketCount);
        }
        if start > end {
            return Err(Error::InvalidTimelineRange);
        }

        let durable = self.inner.storage.query_range_all(start, end).await?;
        let cache = self
            .inner
            .cache
            .lock()
            .await
            .range_inclusive_all(start, end);

        let mut seen = HashSet::new();
        let mut merged = Vec::new();

        for record in durable
            .into_iter()
            .chain(cache.into_iter().map(|r| r.as_ref().clone()))
        {
            let fingerprint = record_fingerprint(&record);
            if seen.insert(fingerprint) {
                merged.push(record);
            }
        }
        merged.sort_by_key(|record| record.timestamp);

        let (ingest_frontier, durable_frontier) = self.aggregate_frontiers().await;
        Ok(build_timeline_summary(
            merged,
            start,
            end,
            buckets,
            ingest_frontier,
            durable_frontier,
        ))
    }

    pub async fn shutdown(self) -> Result<()> {
        info!("shutdown requested");
        let (tx, rx) = oneshot::channel();
        self.inner
            .control_tx
            .send(Command::Shutdown { response: tx })
            .map_err(|_| Error::ControlChannelClosed)?;
        rx.await.map_err(|_| Error::ResponseChannelClosed)?
    }

    async fn aggregate_frontiers(&self) -> (Option<Timestamp>, Option<Timestamp>) {
        let (ingest, durable) = collect_frontiers(&self.inner).await;
        (ingest, durable)
    }
}

#[derive(Clone)]
pub struct SourceHandle {
    inner: Arc<BackendInner>,
    spec: SourceSpec,
    stats_rx: watch::Receiver<SourceStats>,
    live_tx: broadcast::Sender<StreamRecord>,
    _lease: Arc<HandleLease>,
}

impl SourceHandle {
    /// Subscribes to live ingest records for this logical source.
    ///
    /// The receiver is best-effort and does not replay historical records.
    /// If the consumer lags behind sender capacity, `recv` returns lag errors.
    pub fn live_updates(&self) -> broadcast::Receiver<StreamRecord> {
        self.live_tx.subscribe()
    }

    /// Returns the newest visible record for this logical source.
    pub async fn latest(&self) -> Result<Option<StreamRecord>> {
        let durable_frontier = self.stats_snapshot().durable_frontier;
        let cached_latest = {
            let mut cache = self.inner.cache.lock().await;
            let value = cache.latest(&self.spec).map(|r| r.as_ref().clone());
            self.inner.publish_backend_stats(&cache).await;
            value
        };

        if let (Some(record), Some(frontier)) = (&cached_latest, durable_frontier) {
            if record.timestamp > frontier {
                return Ok(Some(record.clone()));
            }
        }

        let durable = self.inner.storage.query_latest(&self.spec).await?;
        Ok(match (cached_latest, durable) {
            (Some(cached), Some(durable)) => {
                if cached.timestamp >= durable.timestamp {
                    Some(cached)
                } else {
                    Some(durable)
                }
            }
            (Some(cached), None) => Some(cached),
            (None, Some(durable)) => Some(durable),
            (None, None) => None,
        })
    }

    /// Returns the newest visible record with timestamp `<= ts`.
    pub async fn before_or_equal(&self, ts: Timestamp) -> Result<Option<StreamRecord>> {
        let cached = {
            let mut cache = self.inner.cache.lock().await;
            let value = cache
                .before_or_equal(&self.spec, ts)
                .map(|r| r.as_ref().clone());
            self.inner.publish_backend_stats(&cache).await;
            value
        };

        let durable = self
            .inner
            .storage
            .query_before_or_equal(&self.spec, ts)
            .await?;
        Ok(match (cached, durable) {
            (Some(cached), Some(durable)) => {
                if cached.timestamp >= durable.timestamp {
                    Some(cached)
                } else {
                    Some(durable)
                }
            }
            (Some(cached), None) => Some(cached),
            (None, Some(durable)) => Some(durable),
            (None, None) => None,
        })
    }

    /// Returns the visible record closest to `ts`, preferring earlier timestamps on ties.
    pub async fn nearest(&self, ts: Timestamp) -> Result<Option<StreamRecord>> {
        let cached = {
            let mut cache = self.inner.cache.lock().await;
            let value = cache.nearest(&self.spec, ts).map(|r| r.as_ref().clone());
            self.inner.publish_backend_stats(&cache).await;
            value
        };

        let durable = self.inner.storage.query_nearest(&self.spec, ts).await?;
        Ok(choose_nearest(ts, cached, durable))
    }

    /// Returns all visible records in the inclusive `[start, end]` range.
    pub async fn range_inclusive(
        &self,
        start: Timestamp,
        end: Timestamp,
    ) -> Result<Vec<StreamRecord>> {
        let durable = self
            .inner
            .storage
            .query_range_inclusive(&self.spec, start, end)
            .await?;

        let durable_frontier = self.stats_snapshot().durable_frontier;
        let cached_tail = self
            .inner
            .cache
            .lock()
            .await
            .range_inclusive_after(&self.spec, start, end, durable_frontier)
            .into_iter()
            .map(|record| record.as_ref().clone());

        let mut merged = durable;
        merged.extend(cached_tail);
        merged.sort_by_key(|record| record.timestamp);
        merged.dedup_by_key(|record| record_fingerprint(record));
        Ok(merged)
    }

    /// Returns bucketed timeline data for this source and range.
    pub async fn timeline(
        &self,
        start: Timestamp,
        end: Timestamp,
        buckets: usize,
    ) -> Result<TimelineSummary> {
        if buckets == 0 {
            return Err(Error::InvalidBucketCount);
        }
        if start > end {
            return Err(Error::InvalidTimelineRange);
        }

        let records = self.range_inclusive(start, end).await?;
        let stats = self.stats_snapshot();

        Ok(build_timeline_summary(
            records,
            start,
            end,
            buckets,
            stats.ingest_frontier,
            stats.durable_frontier,
        ))
    }

    /// Warms the cache for a historical range to improve scrub responsiveness.
    pub async fn prefetch_range(&self, start: Timestamp, end: Timestamp) -> Result<usize> {
        let token = CancellationToken::new();
        self.prefetch_range_cancellable(start, end, 64, &token)
            .await
    }

    /// Warms cache in timestamp chunks and supports cooperative cancellation.
    pub async fn prefetch_range_cancellable(
        &self,
        start: Timestamp,
        end: Timestamp,
        chunks: usize,
        cancel_token: &CancellationToken,
    ) -> Result<usize> {
        if chunks == 0 {
            return Err(Error::InvalidBucketCount);
        }
        if start > end {
            return Err(Error::InvalidTimelineRange);
        }

        let windows = split_timestamp_range(start, end, chunks);
        let mut inserted = 0_usize;

        for (window_start, window_end) in windows {
            if cancel_token.is_cancelled() {
                break;
            }

            let records = self
                .inner
                .storage
                .query_range_inclusive(&self.spec, window_start, window_end)
                .await?;

            if !records.is_empty() {
                let count = records.len();
                let mut cache = self.inner.cache.lock().await;
                for record in records {
                    cache.insert(Arc::new(record));
                }
                self.inner.publish_backend_stats(&cache).await;
                inserted = inserted.saturating_add(count);
            }

            tokio::task::yield_now().await;
        }

        Ok(inserted)
    }

    pub fn stats_snapshot(&self) -> SourceStats {
        self.stats_rx.borrow().clone()
    }

    pub fn stats_watch(&self) -> watch::Receiver<SourceStats> {
        self.stats_rx.clone()
    }
}

struct HandleLease {
    source_key: String,
    control_tx: mpsc::UnboundedSender<Command>,
}

impl Drop for HandleLease {
    fn drop(&mut self) {
        let _ = self.control_tx.send(Command::ReleaseSource {
            source_key: self.source_key.clone(),
        });
    }
}

pub struct StreamDriver {
    inner: Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

impl StreamDriver {
    fn new<F>(future: F) -> Self
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        Self {
            inner: Box::pin(future),
        }
    }
}

impl Future for StreamDriver {
    type Output = Result<()>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

struct BackendInner {
    session: Session,
    node: Node,
    open_mode: OpenMode,
    storage: Storage,
    cache: Arc<Mutex<GlobalCache>>,
    control_tx: mpsc::UnboundedSender<Command>,
    backend_stats_tx: watch::Sender<BackendStats>,
    active_sources: AtomicUsize,
    active_subscribers: AtomicUsize,
    target_namespace_tx: watch::Sender<String>,
    ingest_enabled_tx: watch::Sender<bool>,
    is_closed: AtomicBool,
    writer_enqueued: AtomicU64,
    writer_dequeued: AtomicU64,
    writer_queue_high_watermark: AtomicUsize,
    writer_backpressure_events: AtomicU64,
}

impl BackendInner {
    async fn publish_backend_stats(&self, cache: &GlobalCache) {
        let enqueued = self.writer_enqueued.load(Ordering::SeqCst);
        let dequeued = self.writer_dequeued.load(Ordering::SeqCst);
        let depth = enqueued.saturating_sub(dequeued) as usize;
        self.backend_stats_tx.send_replace(BackendStats {
            active_sources: self.active_sources.load(Ordering::SeqCst),
            active_subscribers: self.active_subscribers.load(Ordering::SeqCst),
            cache: cache.stats(),
            writer_queue_depth: depth,
            writer_queue_high_watermark: self.writer_queue_high_watermark.load(Ordering::SeqCst),
            writer_backpressure_events: self.writer_backpressure_events.load(Ordering::SeqCst),
        });
    }

    async fn publish_backend_stats_from_cache_lock(&self) {
        let cache = self.cache.lock().await;
        self.publish_backend_stats(&cache).await;
    }

    fn note_write_enqueued(&self) {
        let enqueued = self.writer_enqueued.fetch_add(1, Ordering::SeqCst) + 1;
        let dequeued = self.writer_dequeued.load(Ordering::SeqCst);
        let depth = enqueued.saturating_sub(dequeued) as usize;

        loop {
            let high = self.writer_queue_high_watermark.load(Ordering::SeqCst);
            if depth <= high {
                break;
            }
            if self
                .writer_queue_high_watermark
                .compare_exchange(high, depth, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    }

    fn note_write_dequeued(&self) {
        self.writer_dequeued.fetch_add(1, Ordering::SeqCst);
    }

    fn note_backpressure_event(&self) {
        self.writer_backpressure_events
            .fetch_add(1, Ordering::SeqCst);
    }
}

struct SourceRuntime {
    shared: Arc<SourceShared>,
    leases: usize,
    cancel_token: CancellationToken,
    task: Option<tokio::task::JoinHandle<()>>,
}

struct SourceShared {
    spec: SourceSpec,
    stats_tx: watch::Sender<SourceStats>,
    live_tx: broadcast::Sender<StreamRecord>,
}

enum Command {
    AcquireSource {
        spec: SourceSpec,
        response: oneshot::Sender<Result<AcquiredSource>>,
    },
    ReleaseSource {
        source_key: String,
    },
    SetTargetNamespace {
        namespace: String,
        response: oneshot::Sender<Result<()>>,
    },
    SetIngestEnabled {
        enabled: bool,
        response: oneshot::Sender<Result<()>>,
    },
    Shutdown {
        response: oneshot::Sender<Result<()>>,
    },
}

struct AcquiredSource {
    source_key: String,
    spec: SourceSpec,
    stats_rx: watch::Receiver<SourceStats>,
    live_tx: broadcast::Sender<StreamRecord>,
}

struct WriteRequest {
    record: StreamRecord,
    stats_tx: watch::Sender<SourceStats>,
}

async fn run_driver(
    inner: Arc<BackendInner>,
    mut control_rx: mpsc::UnboundedReceiver<Command>,
    write_tx: mpsc::Sender<WriteRequest>,
    write_rx: mpsc::Receiver<WriteRequest>,
    target_namespace_rx: watch::Receiver<String>,
    ingest_enabled_rx: watch::Receiver<bool>,
) -> Result<()> {
    let sources = Arc::new(RwLock::new(HashMap::<String, SourceRuntime>::new()));
    info!("stream driver started");

    let writer_inner = inner.clone();
    let writer_handle = tokio::spawn(async move { run_writer(writer_inner, write_rx).await });

    while let Some(command) = control_rx.recv().await {
        match command {
            Command::AcquireSource { spec, response } => {
                let result = acquire_source(
                    &inner,
                    &sources,
                    spec,
                    &write_tx,
                    &target_namespace_rx,
                    &ingest_enabled_rx,
                )
                .await;
                let _ = response.send(result);
            }
            Command::ReleaseSource { source_key } => {
                release_source(&inner, &sources, &source_key).await;
            }
            Command::SetTargetNamespace {
                namespace,
                response,
            } => {
                info!(%namespace, "driver applying target namespace update");
                let _ = inner.target_namespace_tx.send(namespace);
                let _ = response.send(Ok(()));
            }
            Command::SetIngestEnabled { enabled, response } => {
                info!(enabled, "driver applying ingest enabled update");
                let _ = inner.ingest_enabled_tx.send(enabled);
                let _ = response.send(Ok(()));
            }
            Command::Shutdown { response } => match shutdown_runtime(&inner, &sources).await {
                Ok(()) => {
                    info!("driver shutdown completed");
                    let _ = response.send(Ok(()));
                    break;
                }
                Err(error) => {
                    warn!(%error, "driver shutdown failed");
                    let _ = response.send(Err(error));
                }
            },
        }
    }

    inner.is_closed.store(true, Ordering::SeqCst);
    drop(write_tx);

    let writer_result = writer_handle.await?;
    if let Err(error) = writer_result {
        warn!(%error, "writer terminated with error");
        let _ = shutdown_runtime(&inner, &sources).await;
        return Err(error);
    }

    info!("stream driver stopped");
    Ok(())
}

async fn run_writer(
    inner: Arc<BackendInner>,
    mut write_rx: mpsc::Receiver<WriteRequest>,
) -> Result<()> {
    info!("durable writer started");
    while let Some(request) = write_rx.recv().await {
        let source = source_key(&request.record.source);
        inner.note_write_dequeued();
        match inner.storage.append(request.record).await {
            Ok(durable) => {
                trace!(source = %source, durable_len = durable.len, "record durably persisted");
                update_durable_stats(&request.stats_tx, &durable);
            }
            Err(error) => {
                warn!(source = %source, %error, "failed to append durable record");
                update_last_error(&request.stats_tx, error.to_string());
            }
        }
        inner.publish_backend_stats_from_cache_lock().await;
    }

    info!("durable writer stopped");
    Ok(())
}

async fn acquire_source(
    inner: &Arc<BackendInner>,
    sources: &Arc<RwLock<HashMap<String, SourceRuntime>>>,
    spec: SourceSpec,
    write_tx: &mpsc::Sender<WriteRequest>,
    target_namespace_rx: &watch::Receiver<String>,
    ingest_enabled_rx: &watch::Receiver<bool>,
) -> Result<AcquiredSource> {
    let source_key_value = source_key(&spec);
    debug!(source_key = %source_key_value, "acquire_source command");

    {
        let mut guard = sources.write().await;
        if let Some(runtime) = guard.get_mut(&source_key_value) {
            runtime.leases = runtime.leases.saturating_add(1);
            debug!(
                source_key = %source_key_value,
                leases = runtime.leases,
                "reusing existing deduplicated source runtime",
            );
            return Ok(AcquiredSource {
                source_key: source_key_value,
                spec,
                stats_rx: runtime.shared.stats_tx.subscribe(),
                live_tx: runtime.shared.live_tx.clone(),
            });
        }
    }

    let mut initial_stats = inner.storage.source_stats_snapshot(&spec).await;
    initial_stats.last_error = None;
    let (stats_tx, stats_rx) = watch::channel(initial_stats);
    let (live_tx, _live_rx) = broadcast::channel(LIVE_UPDATES_CAPACITY);

    let shared = Arc::new(SourceShared {
        spec: spec.clone(),
        stats_tx: stats_tx.clone(),
        live_tx,
    });

    let cancel_token = CancellationToken::new();
    let can_ingest = inner.open_mode == OpenMode::ReadWrite && spec.plane != PlaneKind::ExternalRaw;

    let task = if can_ingest {
        debug!(source_key = %source_key_value, "starting ingest worker");
        let worker_inner = inner.clone();
        let worker_shared = shared.clone();
        let worker_cancel = cancel_token.clone();
        let mut target_namespace_rx = target_namespace_rx.clone();
        let mut ingest_enabled_rx = ingest_enabled_rx.clone();
        let write_tx = write_tx.clone();

        Some(tokio::spawn(async move {
            run_ingest_worker(
                worker_inner,
                worker_shared,
                worker_cancel,
                write_tx,
                &mut target_namespace_rx,
                &mut ingest_enabled_rx,
            )
            .await;
        }))
    } else {
        None
    };

    {
        let mut guard = sources.write().await;
        guard.insert(
            source_key_value.clone(),
            SourceRuntime {
                shared: shared.clone(),
                leases: 1,
                cancel_token,
                task,
            },
        );

        inner.active_sources.store(guard.len(), Ordering::SeqCst);
        let subscriber_count = guard
            .values()
            .filter(|runtime| runtime.task.is_some())
            .count();
        inner
            .active_subscribers
            .store(subscriber_count, Ordering::SeqCst);
    }

    inner.publish_backend_stats_from_cache_lock().await;
    debug!(source_key = %source_key_value, "new source runtime registered");

    Ok(AcquiredSource {
        source_key: source_key_value,
        spec,
        stats_rx,
        live_tx: shared.live_tx.clone(),
    })
}

async fn release_source(
    inner: &Arc<BackendInner>,
    sources: &Arc<RwLock<HashMap<String, SourceRuntime>>>,
    source_key_value: &str,
) {
    let mut to_shutdown = None;
    debug!(source_key = %source_key_value, "release_source command");

    {
        let mut guard = sources.write().await;
        if let Some(runtime) = guard.get_mut(source_key_value) {
            if runtime.leases > 1 {
                runtime.leases -= 1;
                debug!(
                    source_key = %source_key_value,
                    leases = runtime.leases,
                    "source lease decremented",
                );
            } else {
                let runtime = guard.remove(source_key_value);
                to_shutdown = runtime;
            }
        }

        inner.active_sources.store(guard.len(), Ordering::SeqCst);
        let subscriber_count = guard
            .values()
            .filter(|runtime| runtime.task.is_some())
            .count();
        inner
            .active_subscribers
            .store(subscriber_count, Ordering::SeqCst);
    }

    if let Some(runtime) = to_shutdown {
        debug!(source_key = %source_key_value, "stopping source runtime after last lease");
        runtime.cancel_token.cancel();
        if let Some(task) = runtime.task {
            task.abort();
        }
    }

    inner.publish_backend_stats_from_cache_lock().await;
}

async fn shutdown_runtime(
    inner: &Arc<BackendInner>,
    sources: &Arc<RwLock<HashMap<String, SourceRuntime>>>,
) -> Result<()> {
    info!("runtime shutdown started");
    let mut guard = sources.write().await;
    let mut runtimes = Vec::with_capacity(guard.len());
    for (_key, runtime) in guard.drain() {
        runtimes.push(runtime);
    }

    inner.active_sources.store(0, Ordering::SeqCst);
    inner.active_subscribers.store(0, Ordering::SeqCst);

    drop(guard);

    for runtime in runtimes {
        runtime.cancel_token.cancel();
        if let Some(task) = runtime.task {
            task.abort();
        }
    }

    inner.storage.shutdown().await?;
    inner.publish_backend_stats_from_cache_lock().await;

    info!("runtime shutdown finished");
    Ok(())
}

async fn run_ingest_worker(
    inner: Arc<BackendInner>,
    shared: Arc<SourceShared>,
    cancel_token: CancellationToken,
    write_tx: mpsc::Sender<WriteRequest>,
    target_namespace_rx: &mut watch::Receiver<String>,
    ingest_enabled_rx: &mut watch::Receiver<bool>,
) {
    if shared.spec.path.scope() == hulkz::Scope::Private && shared.spec.node_override.is_none() {
        warn!("ingest worker rejected private scope source without node override");
        update_last_error(&shared.stats_tx, Error::NodeRequiredForPrivate.to_string());
        return;
    }
    let source = source_key(&shared.spec);
    debug!(source_key = %source, "ingest worker started");

    loop {
        if cancel_token.is_cancelled() {
            debug!(source_key = %source, "ingest worker cancelled");
            return;
        }

        if !*ingest_enabled_rx.borrow() {
            trace!(source_key = %source, "ingest disabled; waiting");
            tokio::select! {
                _ = cancel_token.cancelled() => return,
                changed = ingest_enabled_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
            continue;
        }

        let current_target = target_namespace_rx.borrow().clone();
        let current_effective_namespace = effective_namespace(&shared.spec, &current_target);

        let mut subscriber = match build_subscriber(
            &inner,
            &shared.spec,
            current_effective_namespace.clone(),
            INGEST_CAPACITY,
        )
        .await
        {
            Ok(subscriber) => subscriber,
            Err(error) => {
                warn!(source_key = %source, %error, "failed to build subscriber; retrying");
                update_last_error(&shared.stats_tx, error.to_string());
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        debug!(
            source_key = %source,
            effective_namespace = ?current_effective_namespace,
            "subscriber established",
        );

        loop {
            let rebinding_needed = matches!(
                shared.spec.namespace_binding,
                crate::types::NamespaceBinding::FollowTarget
            );
            tokio::select! {
                _ = cancel_token.cancelled() => return,
                changed = ingest_enabled_rx.changed() => {
                    if changed.is_err() {
                        return;
                    }
                    if !*ingest_enabled_rx.borrow() {
                        break;
                    }
                }
                changed = target_namespace_rx.changed(), if rebinding_needed => {
                    if changed.is_err() {
                        return;
                    }
                    let new_target = target_namespace_rx.borrow().clone();
                    let new_effective = effective_namespace(&shared.spec, &new_target);
                    if new_effective != current_effective_namespace {
                        debug!(source_key = %source, ?new_effective, "rebinding subscriber due to namespace change");
                        break;
                    }
                }
                sample = subscriber.recv_async() => {
                    match sample {
                        Ok(sample) => {
                            let payload: Arc<[u8]> = Arc::from(sample.payload_bytes().into_owned().into_boxed_slice());
                            let record = StreamRecord {
                                source: shared.spec.clone(),
                                effective_namespace: current_effective_namespace.clone(),
                                timestamp: sample.timestamp,
                                encoding: sample.encoding,
                                payload,
                            };

                            {
                                let mut cache = inner.cache.lock().await;
                                cache.insert(Arc::new(record.clone()));
                                inner.publish_backend_stats(&cache).await;
                            }

                            update_ingest_frontier(&shared.stats_tx, record.timestamp);
                            let _ = shared.live_tx.send(record.clone());

                            let request = WriteRequest {
                                record,
                                stats_tx: shared.stats_tx.clone(),
                            };
                            match write_tx.try_send(request) {
                                Ok(()) => {
                                    inner.note_write_enqueued();
                                }
                                Err(TrySendError::Full(request)) => {
                                    inner.note_backpressure_event();
                                    trace!(source_key = %source, "writer queue full; applying backpressure");
                                    if write_tx.send(request).await.is_err() {
                                        warn!(source_key = %source, "writer queue closed while backpressured");
                                        update_last_error(&shared.stats_tx, Error::BackendClosed.to_string());
                                        return;
                                    }
                                    inner.note_write_enqueued();
                                }
                                Err(TrySendError::Closed(_request)) => {
                                    warn!(source_key = %source, "writer queue closed");
                                    update_last_error(&shared.stats_tx, Error::BackendClosed.to_string());
                                    return;
                                }
                            }
                        }
                        Err(error) => {
                            warn!(source_key = %source, %error, "subscriber receive failed; reconnecting");
                            update_last_error(&shared.stats_tx, error.to_string());
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            break;
                        }
                    }
                }
            }
        }
    }
}

enum IngestSubscriber {
    Raw(hulkz::RawSubscriber),
    Param(hulkz::parameter::ParamUpdateRawSubscriber),
}

impl IngestSubscriber {
    async fn recv_async(&mut self) -> std::result::Result<hulkz::Sample, hulkz::Error> {
        match self {
            IngestSubscriber::Raw(subscriber) => subscriber.recv_async().await,
            IngestSubscriber::Param(subscriber) => subscriber.recv_async().await,
        }
    }
}

async fn build_subscriber(
    inner: &BackendInner,
    spec: &SourceSpec,
    effective_namespace: Option<String>,
    capacity: usize,
) -> Result<IngestSubscriber> {
    trace!(
        plane = ?spec.plane,
        scope = %spec.path.scope().as_str(),
        path = %spec.path.path(),
        effective_namespace = ?effective_namespace,
        capacity,
        "building ingest subscriber",
    );
    match spec.plane {
        PlaneKind::Data | PlaneKind::View => {
            let mut builder = inner
                .node
                .subscribe_raw(spec.path.clone())
                .capacity(capacity);
            if spec.plane == PlaneKind::View {
                builder = builder.view();
            }
            if let Some(namespace) = &effective_namespace {
                builder = builder.in_namespace(namespace.clone());
            }
            if let Some(node) = &spec.node_override {
                builder = builder.on_node(node.clone());
            }
            Ok(IngestSubscriber::Raw(builder.build().await?))
        }
        PlaneKind::ParamReadUpdates => {
            let mut access = inner.session.parameter(spec.path.clone());
            if let Some(namespace) = &effective_namespace {
                access = access.in_namespace(namespace.clone());
            }
            if let Some(node) = &spec.node_override {
                access = access.on_node(node);
            }

            Ok(IngestSubscriber::Param(
                access.watch_updates_raw(capacity).await?,
            ))
        }
        PlaneKind::ExternalRaw => Err(Error::InvalidSource),
    }
}

fn update_ingest_frontier(stats_tx: &watch::Sender<SourceStats>, timestamp: Timestamp) {
    let mut snapshot = stats_tx.borrow().clone();
    snapshot.ingest_frontier = Some(match snapshot.ingest_frontier {
        Some(existing) => existing.max(timestamp),
        None => timestamp,
    });
    snapshot.last_error = None;
    let _ = stats_tx.send(snapshot);
}

fn update_durable_stats(stats_tx: &watch::Sender<SourceStats>, durable: &DurableStats) {
    let mut snapshot = stats_tx.borrow().clone();
    snapshot.durable_oldest = durable.oldest;
    snapshot.durable_latest = durable.latest;
    snapshot.durable_len = durable.len;
    snapshot.durable_frontier = durable.latest;
    snapshot.last_error = None;
    let _ = stats_tx.send(snapshot);
}

fn update_last_error(stats_tx: &watch::Sender<SourceStats>, error: String) {
    let mut snapshot = stats_tx.borrow().clone();
    snapshot.last_error = Some(error);
    let _ = stats_tx.send(snapshot);
}

fn choose_nearest(
    target: Timestamp,
    cache: Option<StreamRecord>,
    durable: Option<StreamRecord>,
) -> Option<StreamRecord> {
    match (cache, durable) {
        (Some(cache), Some(durable)) => {
            let cache_diff = target
                .get_time()
                .to_duration()
                .abs_diff(cache.timestamp.get_time().to_duration());
            let durable_diff = target
                .get_time()
                .to_duration()
                .abs_diff(durable.timestamp.get_time().to_duration());

            if cache_diff < durable_diff
                || (cache_diff == durable_diff && cache.timestamp <= durable.timestamp)
            {
                Some(cache)
            } else {
                Some(durable)
            }
        }
        (Some(cache), None) => Some(cache),
        (None, Some(durable)) => Some(durable),
        (None, None) => None,
    }
}

fn build_timeline_summary(
    records: Vec<StreamRecord>,
    start: Timestamp,
    end: Timestamp,
    buckets: usize,
    ingest_frontier: Option<Timestamp>,
    durable_frontier: Option<Timestamp>,
) -> TimelineSummary {
    let start_ns = to_nanos(&start);
    let end_ns = to_nanos(&end).max(start_ns);
    let total_span = end_ns.saturating_sub(start_ns).saturating_add(1);
    let bucket_span = (total_span / buckets as u64).max(1);

    let mut bucket_items = Vec::with_capacity(buckets);
    for index in 0..buckets {
        let bucket_start_ns = start_ns.saturating_add(bucket_span.saturating_mul(index as u64));
        let mut bucket_end_ns = bucket_start_ns.saturating_add(bucket_span.saturating_sub(1));
        if index == buckets - 1 || bucket_end_ns > end_ns {
            bucket_end_ns = end_ns;
        }

        bucket_items.push(TimelineBucket {
            bucket_start: from_nanos(bucket_start_ns),
            bucket_end: from_nanos(bucket_end_ns),
            message_count: 0,
            min_ts: None,
            max_ts: None,
        });
    }

    for record in records {
        let ts_ns = to_nanos(&record.timestamp);
        if ts_ns < start_ns || ts_ns > end_ns {
            continue;
        }

        let mut index = ((ts_ns - start_ns) / bucket_span) as usize;
        if index >= bucket_items.len() {
            index = bucket_items.len().saturating_sub(1);
        }

        if let Some(bucket) = bucket_items.get_mut(index) {
            bucket.message_count = bucket.message_count.saturating_add(1);
            bucket.min_ts = Some(match bucket.min_ts {
                Some(existing) => existing.min(record.timestamp),
                None => record.timestamp,
            });
            bucket.max_ts = Some(match bucket.max_ts {
                Some(existing) => existing.max(record.timestamp),
                None => record.timestamp,
            });
        }
    }

    TimelineSummary {
        buckets: bucket_items,
        ingest_frontier,
        durable_frontier,
    }
}

fn split_timestamp_range(
    start: Timestamp,
    end: Timestamp,
    chunks: usize,
) -> Vec<(Timestamp, Timestamp)> {
    let start_ns = to_nanos(&start);
    let end_ns = to_nanos(&end).max(start_ns);
    let total_span = end_ns.saturating_sub(start_ns).saturating_add(1);
    let chunk_span = (total_span / chunks as u64).max(1);

    let mut windows = Vec::with_capacity(chunks);
    let mut next_start = start_ns;

    for index in 0..chunks {
        if next_start > end_ns {
            break;
        }

        let mut next_end = next_start.saturating_add(chunk_span.saturating_sub(1));
        if index == chunks - 1 || next_end > end_ns {
            next_end = end_ns;
        }

        windows.push((from_nanos(next_start), from_nanos(next_end)));

        if next_end == u64::MAX {
            break;
        }
        next_start = next_end.saturating_add(1);
    }

    windows
}

async fn collect_frontiers(inner: &Arc<BackendInner>) -> (Option<Timestamp>, Option<Timestamp>) {
    let durable_frontier = inner.storage.durable_global_frontier().await;
    let ingest_frontier = inner.cache.lock().await.latest_timestamp();
    (ingest_frontier, durable_frontier)
}

fn record_fingerprint(record: &StreamRecord) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_key(&record.source).hash(&mut hasher);
    record.effective_namespace.hash(&mut hasher);
    to_nanos(&record.timestamp).hash(&mut hasher);
    record.encoding.to_string().hash(&mut hasher);
    record.payload.hash(&mut hasher);
    hasher.finish()
}
