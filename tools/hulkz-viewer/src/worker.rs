use std::{
    collections::BTreeMap,
    num::NonZeroU128,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

mod discovery;

use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz::{ParameterInfo, PublisherInfo, Scope, ScopedPath, Session, SessionInfo, Timestamp};
use hulkz_stream::{
    NamespaceBinding, OpenMode, SourceHandle, SourceSpec, StreamBackend, StreamBackendBuilder,
    StreamRecord,
};
use tokio::sync::{
    broadcast::error::RecvError,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, warn};

use crate::model::{
    DiscoveredParameter, DiscoveredPublisher, DiscoveredSession, DisplayedRecord,
    ParameterReference, SourceBindingInfo, SourceBindingRequest, StreamId, ViewerConfig,
    WorkerCommand, WorkerEvent,
};
use discovery::{
    emit_discovery_snapshot, emit_discovery_snapshot_or_error, insert_discovered_entity,
    reconcile_discovery_snapshot, remove_discovered_entity, restart_discovery_watchers,
    stop_discovery_watchers, DiscoveryEvent, DiscoveryState,
};

const PARAMETER_OPERATION_TIMEOUT: Duration = Duration::from_secs(2);

pub async fn run_worker(
    config: ViewerConfig,
    command_rx: UnboundedReceiver<WorkerCommand>,
    event_tx: UnboundedSender<WorkerEvent>,
    cancellation_token: CancellationToken,
) {
    if let Err(error) =
        run_worker_inner(config, command_rx, event_tx.clone(), cancellation_token).await
    {
        warn!("worker terminated with error: {error:?}");
        send_error(&event_tx, format!("worker terminated: {error:#}"));
    }
}

struct WorkerStreamContext {
    source: SourceHandle,
    generation: u64,
    live_cancel: CancellationToken,
    live_task: tokio::task::JoinHandle<()>,
}

enum WorkerInternalEvent {
    Record {
        stream_id: StreamId,
        generation: u64,
        record: StreamRecord,
    },
    Lagged {
        stream_id: StreamId,
        generation: u64,
        skipped: u64,
    },
    Closed {
        stream_id: StreamId,
        generation: u64,
    },
}

async fn run_worker_inner(
    config: ViewerConfig,
    mut command_rx: UnboundedReceiver<WorkerCommand>,
    event_tx: UnboundedSender<WorkerEvent>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    info!(
        namespace = %config.namespace,
        source_expression = %config.source_expression,
        "worker starting"
    );
    let session = Session::create(&config.namespace).await.wrap_err_with(|| {
        format!(
            "failed to create hulkz session for namespace {}",
            config.namespace
        )
    })?;
    let discovery_session = session.clone();

    let storage_path = storage_path_for_config(&config);
    info!(path = %storage_path.display(), "opening stream backend");
    let (backend, driver) = StreamBackendBuilder::new(session)
        .open_mode(OpenMode::ReadWrite)
        .storage_path(storage_path.clone())
        .build()
        .await
        .wrap_err_with(|| {
            format!(
                "failed to build stream backend at {}",
                storage_path.display()
            )
        })?;
    let mut driver_task = tokio::spawn(driver);
    info!("stream driver spawned");

    let (internal_event_tx, mut internal_event_rx) =
        tokio::sync::mpsc::unbounded_channel::<WorkerInternalEvent>();
    let mut streams: BTreeMap<StreamId, WorkerStreamContext> = BTreeMap::new();
    let mut next_generation: u64 = 1;

    event_tx
        .send(WorkerEvent::Ready)
        .map_err(|_| eyre!("failed to send Ready event: worker event channel closed"))?;
    info!("worker ready; awaiting stream bind commands");

    let mut stats_interval = tokio::time::interval(config.poll_interval);
    stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut discovery_reconcile_interval =
        tokio::time::interval(config.discovery_reconcile_interval);
    discovery_reconcile_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut discovery_namespace = String::new();
    let mut discovery = DiscoveryState::new();
    let (discovery_event_tx, mut discovery_event_rx) =
        tokio::sync::mpsc::unbounded_channel::<DiscoveryEvent>();

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                info!("worker cancellation requested");
                break;
            }
            command = command_rx.recv() => {
                match command {
                    Some(WorkerCommand::SetIngestEnabled(enabled)) => {
                        info!(enabled, "setting ingest state");
                        if let Err(error) = backend
                            .set_ingest_enabled(enabled)
                            .await
                            .wrap_err_with(|| format!("failed to set ingest state to {enabled}"))
                        {
                            send_error(&event_tx, format!("{error:#}"));
                        }
                    }
                    Some(WorkerCommand::SetDiscoveryNamespace(namespace)) => {
                        let namespace = namespace.trim().to_string();
                        if namespace == discovery_namespace {
                            continue;
                        }
                        if namespace.is_empty() {
                            discovery_namespace.clear();
                            stop_discovery_watchers(&mut discovery.cancel, &mut discovery.tasks);
                            discovery.cancel = CancellationToken::new();
                            discovery.publishers.clear();
                            discovery.parameters.clear();
                            discovery.sessions.clear();
                            if let Err(error) = emit_discovery_snapshot(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            ) {
                                send_error(&event_tx, format!("{error:#}"));
                            }
                            info!("cleared discovery namespace; discovery disabled until set");
                            continue;
                        }
                        discovery_namespace = namespace;
                        info!(namespace = %discovery_namespace, "updating discovery namespace");
                        if let Err(error) = restart_discovery_watchers(
                            &discovery_session,
                            &discovery_namespace,
                            &discovery_event_tx,
                            &event_tx,
                            &mut discovery,
                        )
                        .await
                        {
                            send_error(&event_tx, format!("{error:#}"));
                        }
                    }
                    Some(WorkerCommand::BindStream { stream_id, request }) => {
                        info!(
                            stream_id,
                            namespace = %request.namespace,
                            plane = ?request.plane,
                            path_expression = %request.path_expression,
                            "binding stream source",
                        );
                        match bind_source(&backend, &request).await {
                            Ok((new_source, binding_label, binding)) => {
                                if let Some(existing) = streams.remove(&stream_id) {
                                    stop_stream_context(existing);
                                }

                                let generation = next_generation;
                                next_generation = next_generation.saturating_add(1);
                                let live_cancel = CancellationToken::new();
                                let live_task = spawn_live_updates_task(
                                    stream_id,
                                    generation,
                                    new_source.live_updates(),
                                    internal_event_tx.clone(),
                                    live_cancel.clone(),
                                );
                                let _ = event_tx.send(WorkerEvent::SourceBound {
                                    stream_id,
                                    label: binding_label,
                                    binding,
                                });
                                if let Err(error) =
                                    emit_history_snapshot(stream_id, &new_source, &event_tx).await
                                {
                                    send_error(&event_tx, format!("{error:#}"));
                                }
                                streams.insert(
                                    stream_id,
                                    WorkerStreamContext {
                                        source: new_source,
                                        generation,
                                        live_cancel,
                                        live_task,
                                    },
                                );
                            }
                            Err(error) => {
                                send_error(&event_tx, format!("{error:#}"));
                            }
                        }
                    }
                    Some(WorkerCommand::RemoveStream { stream_id }) => {
                        if let Some(existing) = streams.remove(&stream_id) {
                            info!(stream_id, "removing stream binding");
                            stop_stream_context(existing);
                        }
                    }
                    Some(WorkerCommand::ReadParameter(target)) => {
                        match read_parameter_value(&discovery_session, &target).await {
                            Ok(value_pretty) => {
                                let _ = event_tx.send(WorkerEvent::ParameterValueLoaded {
                                    target,
                                    value_pretty,
                                });
                            }
                            Err(error) => {
                                send_error(&event_tx, format!("{error:#}"));
                            }
                        }
                    }
                    Some(WorkerCommand::SetParameter { target, value_json }) => {
                        match write_parameter_value(&discovery_session, &target, &value_json).await {
                            Ok(message) => {
                                let _ = event_tx.send(WorkerEvent::ParameterWriteResult {
                                    target: target.clone(),
                                    success: true,
                                    message,
                                });
                                match read_parameter_value(&discovery_session, &target).await {
                                    Ok(value_pretty) => {
                                        let _ = event_tx.send(WorkerEvent::ParameterValueLoaded {
                                            target,
                                            value_pretty,
                                        });
                                    }
                                    Err(error) => {
                                        send_error(&event_tx, format!("{error:#}"));
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = event_tx.send(WorkerEvent::ParameterWriteResult {
                                    target,
                                    success: false,
                                    message: format!("{error:#}"),
                                });
                            }
                        }
                    }
                    Some(WorkerCommand::SetScrubAnchor { stream_id, anchor_nanos }) => {
                        debug!(stream_id, anchor_nanos, "received scrub anchor");
                        if let Some(context) = streams.get(&stream_id) {
                            match resolve_record_at_anchor(&context.source, anchor_nanos).await {
                                Ok(record) => {
                                    let _ = event_tx.send(WorkerEvent::AnchorRecord {
                                        stream_id,
                                        anchor_nanos,
                                        record,
                                    });
                                }
                                Err(error) => {
                                    send_error(&event_tx, format!("{error:#}"));
                                }
                            }
                            if let Err(error) = apply_scrub_anchor(
                                &backend,
                                &context.source,
                                &config,
                                anchor_nanos,
                            )
                            .await
                            {
                                send_error(&event_tx, format!("{error:#}"));
                            }
                        }
                    }
                    Some(WorkerCommand::Shutdown) | None => {
                        info!("worker shutdown command received");
                        break;
                    }
                }
            }
            Some(internal_event) = internal_event_rx.recv() => {
                match internal_event {
                    WorkerInternalEvent::Record { stream_id, generation, record } => {
                        let is_current = streams
                            .get(&stream_id)
                            .map(|ctx| ctx.generation == generation)
                            .unwrap_or(false);
                        if !is_current {
                            continue;
                        }
                        trace!(
                            stream_id,
                            timestamp_nanos = record.timestamp.get_time().as_nanos(),
                            payload_bytes = record.payload.len(),
                            "live record received"
                        );
                        event_tx
                            .send(WorkerEvent::RecordsAppended {
                                stream_id,
                                records: vec![stream_record_to_displayed_record(&record)],
                            })
                            .map_err(|_| eyre!("failed to send record event: worker event channel closed"))?;
                    }
                    WorkerInternalEvent::Lagged {
                        stream_id,
                        generation,
                        skipped,
                    } => {
                        let is_current = streams
                            .get(&stream_id)
                            .map(|ctx| ctx.generation == generation)
                            .unwrap_or(false);
                        if !is_current {
                            continue;
                        }
                        warn!(stream_id, skipped, "live updates receiver lagged");
                        send_error(
                            &event_tx,
                            format!("stream {stream_id} live updates lagged; skipped {skipped} records"),
                        );
                    }
                    WorkerInternalEvent::Closed { stream_id, generation } => {
                        let is_current = streams
                            .get(&stream_id)
                            .map(|ctx| ctx.generation == generation)
                            .unwrap_or(false);
                        if is_current {
                            warn!(stream_id, "live updates channel closed unexpectedly");
                            send_error(
                                &event_tx,
                                format!("stream {stream_id} live updates channel closed"),
                            );
                        }
                    }
                }
            }
            _ = stats_interval.tick() => {
                trace!("publishing stats snapshot");
                for (stream_id, context) in &streams {
                    event_tx
                        .send(WorkerEvent::StreamStats {
                            stream_id: *stream_id,
                            source: Box::new(context.source.stats_snapshot()),
                        })
                        .map_err(|_| eyre!("failed to send stream stats event: worker event channel closed"))?;
                }
                event_tx
                    .send(WorkerEvent::BackendStats {
                        backend: Box::new(backend.stats_snapshot()),
                    })
                    .map_err(|_| eyre!("failed to send backend stats event: worker event channel closed"))?;
            }
            Some(discovery_event) = discovery_event_rx.recv() => {
                match discovery_event {
                    DiscoveryEvent::PublisherJoined(publisher) => {
                        if insert_discovered_entity(&mut discovery.publishers, publisher) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::PublisherLeft(publisher) => {
                        if remove_discovered_entity(&mut discovery.publishers, &publisher) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::ParameterJoined(parameter) => {
                        if insert_discovered_entity(&mut discovery.parameters, parameter) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::ParameterLeft(parameter) => {
                        if remove_discovered_entity(&mut discovery.parameters, &parameter) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::SessionJoined(session_info) => {
                        if insert_discovered_entity(&mut discovery.sessions, session_info) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::SessionLeft(session_info) => {
                        if remove_discovered_entity(&mut discovery.sessions, &session_info) {
                            emit_discovery_snapshot_or_error(
                                &event_tx,
                                &discovery.publishers,
                                &discovery.parameters,
                                &discovery.sessions,
                            );
                        }
                    }
                    DiscoveryEvent::WatchFault(message) => {
                        send_error(&event_tx, message);
                    }
                }
            }
            _ = discovery_reconcile_interval.tick() => {
                if discovery_namespace.is_empty() {
                    continue;
                }
                if let Err(error) = reconcile_discovery_snapshot(
                    &discovery_session,
                    &discovery_namespace,
                    &event_tx,
                    &mut discovery.publishers,
                    &mut discovery.parameters,
                    &mut discovery.sessions,
                ).await {
                    send_error(&event_tx, format!("{error:#}"));
                }
            }
        }
    }

    stop_discovery_watchers(&mut discovery.cancel, &mut discovery.tasks);
    for (_, context) in streams {
        stop_stream_context(context);
    }

    info!("worker shutting down");
    shutdown_worker(backend, &mut driver_task, &event_tx)
        .await
        .wrap_err("failed during worker shutdown")?;
    info!("worker stopped");
    Ok(())
}

async fn emit_history_snapshot(
    stream_id: StreamId,
    source: &SourceHandle,
    event_tx: &UnboundedSender<WorkerEvent>,
) -> Result<()> {
    let stats = source.stats_snapshot();
    let (Some(start), Some(end)) = (stats.durable_oldest, stats.durable_latest) else {
        debug!("no durable history available for source");
        return Ok(());
    };

    let records = source
        .range_inclusive(start, end)
        .await
        .wrap_err("failed to query durable source history")?;
    if records.is_empty() {
        return Ok(());
    }

    let displayed_records = records
        .iter()
        .map(stream_record_to_displayed_record)
        .collect::<Vec<_>>();
    info!(
        stream_id,
        count = displayed_records.len(),
        "emitting source history snapshot"
    );
    event_tx
        .send(WorkerEvent::RecordsAppended {
            stream_id,
            records: displayed_records,
        })
        .map_err(|_| eyre!("failed to send history snapshot event: worker event channel closed"))?;
    Ok(())
}

fn to_discovered_publisher(info: PublisherInfo) -> DiscoveredPublisher {
    let path_expression = scoped_path_expression(info.scope, &info.path, Some(&info.node));
    DiscoveredPublisher {
        namespace: info.namespace,
        node: info.node,
        path_expression,
    }
}

fn to_discovered_parameter(info: ParameterInfo) -> DiscoveredParameter {
    let path_expression = scoped_path_expression(info.scope, &info.path, Some(&info.node));
    DiscoveredParameter {
        namespace: info.namespace,
        node: info.node,
        path_expression,
    }
}

fn to_discovered_session(info: SessionInfo) -> DiscoveredSession {
    DiscoveredSession {
        namespace: info.namespace,
        id: info.id,
    }
}

async fn apply_scrub_anchor(
    backend: &StreamBackend,
    source: &SourceHandle,
    config: &ViewerConfig,
    anchor_nanos: u64,
) -> Result<()> {
    let window_radius_nanos = config.scrub_window_radius.as_nanos() as u64;
    let prefetch_radius_nanos = config.scrub_prefetch_radius.as_nanos() as u64;

    let window_start = anchor_nanos.saturating_sub(window_radius_nanos);
    let window_end = anchor_nanos.saturating_add(window_radius_nanos);
    backend
        .set_scrub_window(Some((
            timestamp_from_nanos(window_start),
            timestamp_from_nanos(window_end),
        )))
        .await
        .wrap_err_with(|| {
            format!("failed to update scrub window [{window_start}, {window_end}]")
        })?;
    debug!(
        window_start_nanos = window_start,
        window_end_nanos = window_end,
        "updated scrub window"
    );

    let prefetch_start = anchor_nanos.saturating_sub(prefetch_radius_nanos);
    let prefetch_end = anchor_nanos.saturating_add(prefetch_radius_nanos);
    let _ = source
        .prefetch_range(
            timestamp_from_nanos(prefetch_start),
            timestamp_from_nanos(prefetch_end),
        )
        .await
        .wrap_err_with(|| {
            format!("failed to prefetch scrub range [{prefetch_start}, {prefetch_end}]")
        })?;
    debug!(
        prefetch_start_nanos = prefetch_start,
        prefetch_end_nanos = prefetch_end,
        "prefetched scrub range"
    );

    Ok(())
}

async fn resolve_record_at_anchor(
    source: &SourceHandle,
    anchor_nanos: u64,
) -> Result<Option<DisplayedRecord>> {
    source
        .before_or_equal(timestamp_from_nanos(anchor_nanos))
        .await
        .map(|record| record.as_ref().map(stream_record_to_displayed_record))
        .wrap_err_with(|| {
            format!("failed to query source before_or_equal({anchor_nanos}) for scrub anchor")
        })
}

async fn shutdown_worker(
    backend: StreamBackend,
    driver_task: &mut tokio::task::JoinHandle<hulkz_stream::Result<()>>,
    event_tx: &UnboundedSender<WorkerEvent>,
) -> Result<()> {
    backend
        .shutdown()
        .await
        .wrap_err("backend shutdown failed")?;

    match tokio::time::timeout(Duration::from_secs(2), driver_task).await {
        Ok(join_result) => match join_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                return Err(eyre!("stream driver terminated with error: {error}"));
            }
            Err(error) => {
                return Err(eyre!("stream driver join failed: {error}"));
            }
        },
        Err(_) => {
            return Err(eyre!("stream driver shutdown timed out"));
        }
    }

    let _ = event_tx;
    Ok(())
}

async fn bind_source(
    backend: &StreamBackend,
    request: &SourceBindingRequest,
) -> Result<(SourceHandle, String, SourceBindingInfo)> {
    let spec = source_spec_from_request(request)?;
    let label = source_label(request, &spec);
    let source = backend
        .source(spec.clone())
        .await
        .wrap_err_with(|| format!("failed to acquire source {label}"))?;
    info!(source = %label, "source bound");
    let binding = SourceBindingInfo {
        namespace: request.namespace.trim().to_string(),
        path_expression: request.path_expression.trim().to_string(),
    };
    Ok((source, label, binding))
}

fn source_spec_from_request(request: &SourceBindingRequest) -> Result<SourceSpec> {
    let namespace = request.namespace.trim();
    if namespace.is_empty() {
        return Err(eyre!("namespace must not be empty"));
    }

    let (path, node_override) = parse_source_path_expression(&request.path_expression)?;
    if path.scope() == Scope::Private && node_override.is_none() {
        return Err(eyre!(
            "private source requires node override; use ~<node>/<path> syntax"
        ));
    }

    Ok(SourceSpec {
        plane: request.plane,
        path,
        node_override,
        namespace_binding: NamespaceBinding::Pinned(namespace.to_string()),
    })
}

fn parse_source_path_expression(input: &str) -> Result<(ScopedPath, Option<String>)> {
    let expression = input.trim();
    if expression.is_empty() {
        return Err(eyre!("path expression must not be empty"));
    }

    if let Some(rest) = expression.strip_prefix('~') {
        if let Some(path) = rest.strip_prefix('/') {
            if path.is_empty() {
                return Err(eyre!("private path must not be empty"));
            }
            return Ok((ScopedPath::new(Scope::Private, path), None));
        }

        let (node, private_path) = rest
            .split_once('/')
            .ok_or_else(|| eyre!("invalid private syntax; use ~<node>/<path>"))?;
        if node.is_empty() || private_path.is_empty() {
            return Err(eyre!("invalid private syntax; use ~<node>/<path>"));
        }
        return Ok((
            ScopedPath::new(Scope::Private, private_path),
            Some(node.to_string()),
        ));
    }

    let scoped_path = ScopedPath::parse(expression);
    if scoped_path.path().is_empty() {
        return Err(eyre!("path must not be empty"));
    }
    Ok((scoped_path, None))
}

fn source_label(request: &SourceBindingRequest, spec: &SourceSpec) -> String {
    let scope_path = scoped_path_expression(
        spec.path.scope(),
        spec.path.path(),
        spec.node_override.as_deref(),
    );
    format!(
        "namespace={} plane={:?} path={}",
        request.namespace.trim(),
        request.plane,
        scope_path
    )
}

async fn read_parameter_value(session: &Session, target: &ParameterReference) -> Result<String> {
    let (namespace, node, path_expression) = parameter_access_parts(target)?;
    let read_future = async {
        let mut replies = session
            .parameter(path_expression.as_str())
            .on_node(&node)
            .in_namespace(namespace.clone())
            .get::<serde_json::Value>()
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to start parameter read for {} on node {} in namespace {}",
                    target.path_expression, node, namespace
                )
            })?;

        if let Some(reply) = replies.recv_async().await {
            return match reply {
                Ok(value) => {
                    let pretty =
                        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                    Ok(pretty)
                }
                Err(error) => Err(eyre!("parameter read failed: {error}")),
            };
        }

        Err(eyre!(
            "parameter read returned no replies for {}",
            target.path_expression
        ))
    };

    tokio::time::timeout(PARAMETER_OPERATION_TIMEOUT, read_future)
        .await
        .map_err(|_| {
            eyre!(
                "parameter read timed out after {:?} for {}",
                PARAMETER_OPERATION_TIMEOUT,
                target.path_expression
            )
        })?
}

async fn write_parameter_value(
    session: &Session,
    target: &ParameterReference,
    value_json: &str,
) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(value_json)
        .wrap_err("parameter value must be valid JSON before apply")?;
    let (namespace, node, path_expression) = parameter_access_parts(target)?;

    let write_future = async {
        let mut replies = session
            .parameter(path_expression.as_str())
            .on_node(&node)
            .in_namespace(namespace.clone())
            .set(&value)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to send parameter write for {} on node {} in namespace {}",
                    target.path_expression, node, namespace
                )
            })?;

        match replies.recv_async().await {
            Some(Ok(())) => Ok("Parameter apply succeeded".to_string()),
            Some(Err(error)) => Err(eyre!("parameter write rejected: {error}")),
            None => Err(eyre!(
                "parameter write returned no replies for {}",
                target.path_expression
            )),
        }
    };

    tokio::time::timeout(PARAMETER_OPERATION_TIMEOUT, write_future)
        .await
        .map_err(|_| {
            eyre!(
                "parameter write timed out after {:?} for {}",
                PARAMETER_OPERATION_TIMEOUT,
                target.path_expression
            )
        })?
}

fn parameter_access_parts(target: &ParameterReference) -> Result<(String, String, String)> {
    let namespace = target.namespace.trim();
    if namespace.is_empty() {
        return Err(eyre!("parameter namespace must not be empty"));
    }
    let node = target.node.trim();
    if node.is_empty() {
        return Err(eyre!("parameter node must not be empty"));
    }

    let (path, node_override) = parse_source_path_expression(&target.path_expression)?;
    if path.scope() == Scope::Private && node_override.is_none() {
        return Err(eyre!(
            "private parameter requires node override; use ~<node>/<path> syntax"
        ));
    }

    let canonical_path = match path.scope() {
        Scope::Private => scoped_path_expression(path.scope(), path.path(), None),
        _ => scoped_path_expression(path.scope(), path.path(), Some(node)),
    };

    Ok((namespace.to_string(), node.to_string(), canonical_path))
}

fn spawn_live_updates_task(
    stream_id: StreamId,
    generation: u64,
    mut live_updates: tokio::sync::broadcast::Receiver<StreamRecord>,
    internal_event_tx: UnboundedSender<WorkerInternalEvent>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }
                live = live_updates.recv() => {
                    match live {
                        Ok(record) => {
                            if internal_event_tx
                                .send(WorkerInternalEvent::Record {
                                    stream_id,
                                    generation,
                                    record,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(RecvError::Lagged(skipped)) => {
                            if internal_event_tx
                                .send(WorkerInternalEvent::Lagged {
                                    stream_id,
                                    generation,
                                    skipped,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(RecvError::Closed) => {
                            let _ = internal_event_tx.send(WorkerInternalEvent::Closed {
                                stream_id,
                                generation,
                            });
                            break;
                        }
                    }
                }
            }
        }
    })
}

fn stop_stream_context(context: WorkerStreamContext) {
    context.live_cancel.cancel();
    context.live_task.abort();
    drop(context.source);
}

fn send_error(event_tx: &UnboundedSender<WorkerEvent>, message: String) {
    warn!(%message, "worker error");
    let _ = event_tx.send(WorkerEvent::Error(message));
}

fn scoped_path_expression(scope: Scope, path: &str, private_node: Option<&str>) -> String {
    match scope {
        Scope::Global => format!("/{path}"),
        Scope::Local => path.to_string(),
        Scope::Private => match private_node {
            Some(node) => format!("~{node}/{path}"),
            None => format!("~/{path}"),
        },
    }
}

fn session_storage_path() -> PathBuf {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("hulkz-viewer-{}-{run_id}", std::process::id()))
}

fn storage_path_for_config(config: &ViewerConfig) -> PathBuf {
    config
        .storage_path
        .clone()
        .unwrap_or_else(session_storage_path)
}

fn timestamp_from_nanos(nanos: u64) -> Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

fn to_nanos(timestamp: &Timestamp) -> u64 {
    timestamp.get_time().as_nanos()
}

fn stream_record_to_displayed_record(record: &StreamRecord) -> DisplayedRecord {
    let encoding = record.encoding.to_string();
    let (json_pretty, raw_fallback) = decode_payload(&encoding, &record.payload);

    DisplayedRecord {
        timestamp_nanos: to_nanos(&record.timestamp),
        json_pretty,
        raw_fallback,
    }
}

fn decode_payload(encoding: &str, payload: &[u8]) -> (Option<String>, Option<String>) {
    if encoding.to_ascii_lowercase().contains("json") {
        match serde_json::from_slice::<serde_json::Value>(payload) {
            Ok(value) => {
                let pretty =
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                return (Some(pretty), None);
            }
            Err(_) => {
                return (None, Some(text_or_hex_fallback(payload)));
            }
        }
    }

    (None, Some(text_or_hex_fallback(payload)))
}

fn text_or_hex_fallback(payload: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(payload) {
        return text.to_string();
    }

    let preview_len = payload.len().min(64);
    let hex_preview = payload[..preview_len]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "{} bytes (hex preview: {}{})",
        payload.len(),
        hex_preview,
        if payload.len() > preview_len {
            " ..."
        } else {
            ""
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        decode_payload, parameter_access_parts, parse_source_path_expression, run_worker,
        session_storage_path, to_discovered_parameter, to_discovered_publisher,
    };
    use crate::model::{ParameterReference, ViewerConfig, WorkerCommand, WorkerEvent};
    use hulkz::{ParameterInfo, PublisherInfo, Scope, Session};
    use hulkz_stream::PlaneKind;
    use serde::Serialize;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    #[derive(Serialize)]
    struct TestOdometry {
        x: f64,
        y: f64,
        theta: f64,
    }

    fn unique_namespace(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{prefix}-{nanos}")
    }

    async fn recv_event_matching<F>(
        rx: &mut mpsc::UnboundedReceiver<WorkerEvent>,
        timeout: Duration,
        mut predicate: F,
    ) -> Option<WorkerEvent>
    where
        F: FnMut(&WorkerEvent) -> bool,
    {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                return None;
            }
            let remaining = deadline - now;
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(event)) => {
                    if predicate(&event) {
                        return Some(event);
                    }
                }
                Ok(None) | Err(_) => return None,
            }
        }
    }

    async fn shutdown_worker_task(
        command_tx: mpsc::UnboundedSender<WorkerCommand>,
        cancel: CancellationToken,
        task: tokio::task::JoinHandle<()>,
    ) {
        let _ = command_tx.send(WorkerCommand::Shutdown);
        cancel.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(3), task).await;
    }

    #[test]
    fn valid_json_decodes_to_pretty_output() {
        let payload = br#"{"x":1,"y":2}"#;
        let (json, fallback) = decode_payload("application/json", payload);

        assert!(json.is_some());
        assert!(fallback.is_none());
        assert!(json.unwrap().contains("\"x\": 1"));
    }

    #[test]
    fn malformed_json_falls_back_to_text() {
        let payload = br#"{not-json"#;
        let (json, fallback) = decode_payload("application/json", payload);

        assert!(json.is_none());
        assert!(fallback.is_some());
    }

    #[test]
    fn non_json_payload_uses_hex_fallback_when_not_utf8() {
        let payload = [0xff, 0x10, 0xab, 0x00];
        let (json, fallback) = decode_payload("application/cdr", &payload);

        assert!(json.is_none());
        let fallback = fallback.expect("fallback should exist");
        assert!(fallback.contains("hex preview"));
        assert!(fallback.contains("ff"));
    }

    #[test]
    fn parses_local_path_expression() {
        let (path, node) = parse_source_path_expression("odometry").expect("must parse");
        assert_eq!(path.scope(), Scope::Local);
        assert_eq!(path.path(), "odometry");
        assert!(node.is_none());
    }

    #[test]
    fn parses_global_path_expression() {
        let (path, node) = parse_source_path_expression("/fleet/topic").expect("must parse");
        assert_eq!(path.scope(), Scope::Global);
        assert_eq!(path.path(), "fleet/topic");
        assert!(node.is_none());
    }

    #[test]
    fn parses_private_node_override_expression() {
        let (path, node) = parse_source_path_expression("~planner/debug").expect("must parse");
        assert_eq!(path.scope(), Scope::Private);
        assert_eq!(path.path(), "debug");
        assert_eq!(node.as_deref(), Some("planner"));
    }

    #[test]
    fn discovered_private_publisher_uses_node_prefixed_expression() {
        let info = PublisherInfo {
            namespace: "demo".to_string(),
            node: "planner".to_string(),
            scope: Scope::Private,
            path: "debug/topic".to_string(),
        };
        let publisher = to_discovered_publisher(info);
        assert_eq!(publisher.path_expression, "~planner/debug/topic");
    }

    #[test]
    fn discovered_global_parameter_uses_global_expression() {
        let info = ParameterInfo {
            namespace: "demo".to_string(),
            node: "config".to_string(),
            scope: Scope::Global,
            path: "fleet/id".to_string(),
        };
        let parameter = to_discovered_parameter(info);
        assert_eq!(parameter.path_expression, "/fleet/id");
    }

    #[test]
    fn parameter_access_parts_normalize_private_path() {
        let target = ParameterReference {
            namespace: "demo".to_string(),
            node: "planner".to_string(),
            path_expression: "~planner/debug/level".to_string(),
        };
        let (namespace, node, path) = parameter_access_parts(&target).expect("must parse");
        assert_eq!(namespace, "demo");
        assert_eq!(node, "planner");
        assert_eq!(path, "~/debug/level");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn integration_worker_binds_and_receives_live_updates() {
        let namespace = unique_namespace("viewer-live");
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let config = ViewerConfig {
            namespace: namespace.clone(),
            source_expression: "odometry".to_string(),
            storage_path: Some(session_storage_path()),
            ..ViewerConfig::default()
        };
        let worker_task = tokio::spawn(run_worker(config, command_rx, event_tx, cancel.clone()));

        let ready = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::Ready)
        })
        .await;
        assert!(ready.is_some(), "worker did not emit Ready");

        command_tx
            .send(WorkerCommand::BindStream {
                stream_id: 1,
                request: crate::model::SourceBindingRequest {
                    namespace: namespace.clone(),
                    plane: PlaneKind::View,
                    path_expression: "odometry".to_string(),
                },
            })
            .expect("bind command send failed");

        let bound = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::SourceBound { stream_id: 1, .. })
        })
        .await;
        assert!(bound.is_some(), "worker did not emit SourceBound");

        let publisher_session = Session::create(&namespace)
            .await
            .expect("publisher session create failed");
        let publisher_node = publisher_session
            .create_node("viewer-test-publisher")
            .build()
            .await
            .expect("publisher node build failed");
        let publisher = publisher_node
            .advertise::<TestOdometry>("odometry")
            .build()
            .await
            .expect("publisher build failed");

        for i in 0..5 {
            publisher
                .put(
                    &TestOdometry {
                        x: i as f64,
                        y: i as f64,
                        theta: i as f64,
                    },
                    &publisher_session.now(),
                )
                .await
                .expect("publish failed");
            tokio::time::sleep(Duration::from_millis(40)).await;
        }

        let records_event = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 1,
                    records
                } if !records.is_empty()
            )
        })
        .await;
        assert!(
            records_event.is_some(),
            "worker did not emit RecordsAppended for live data"
        );

        shutdown_worker_task(command_tx, cancel, worker_task).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn integration_discovery_snapshot_includes_sessions() {
        let namespace = unique_namespace("viewer-discovery");
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let config = ViewerConfig {
            namespace: namespace.clone(),
            source_expression: "odometry".to_string(),
            storage_path: Some(session_storage_path()),
            ..ViewerConfig::default()
        };
        let worker_task = tokio::spawn(run_worker(config, command_rx, event_tx, cancel.clone()));

        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::Ready)
        })
        .await
        .expect("worker did not emit Ready");

        command_tx
            .send(WorkerCommand::SetDiscoveryNamespace(namespace.clone()))
            .expect("set discovery namespace failed");

        let discovery = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::DiscoverySnapshot { sessions, .. } if !sessions.is_empty())
        })
        .await;
        assert!(
            discovery.is_some(),
            "expected discovery snapshot with at least one session"
        );

        shutdown_worker_task(command_tx, cancel, worker_task).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn integration_rebind_replays_history_snapshot() {
        let namespace = unique_namespace("viewer-rebind");
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let config = ViewerConfig {
            namespace: namespace.clone(),
            source_expression: "odometry".to_string(),
            storage_path: Some(session_storage_path()),
            ..ViewerConfig::default()
        };
        let worker_task = tokio::spawn(run_worker(config, command_rx, event_tx, cancel.clone()));

        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::Ready)
        })
        .await
        .expect("worker did not emit Ready");

        command_tx
            .send(WorkerCommand::BindStream {
                stream_id: 2,
                request: crate::model::SourceBindingRequest {
                    namespace: namespace.clone(),
                    plane: PlaneKind::View,
                    path_expression: "odometry".to_string(),
                },
            })
            .expect("bind command failed");
        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::SourceBound { stream_id: 2, .. })
        })
        .await
        .expect("source was not bound");

        let publisher_session = Session::create(&namespace)
            .await
            .expect("publisher session create failed");
        let publisher_node = publisher_session
            .create_node("viewer-rebind-publisher")
            .build()
            .await
            .expect("publisher node build failed");
        let publisher = publisher_node
            .advertise::<TestOdometry>("odometry")
            .build()
            .await
            .expect("publisher build failed");

        for i in 0..8 {
            publisher
                .put(
                    &TestOdometry {
                        x: i as f64,
                        y: i as f64,
                        theta: i as f64,
                    },
                    &publisher_session.now(),
                )
                .await
                .expect("publish failed");
            tokio::time::sleep(Duration::from_millis(35)).await;
        }

        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 2,
                    records
                } if !records.is_empty()
            )
        })
        .await
        .expect("no live records before rebind");

        let durable_ready = recv_event_matching(&mut event_rx, Duration::from_secs(8), |event| {
            matches!(
                event,
                WorkerEvent::StreamStats { stream_id: 2, source } if source.durable_len > 0
            )
        })
        .await;
        assert!(
            durable_ready.is_some(),
            "expected durable history before rebind"
        );

        command_tx
            .send(WorkerCommand::RemoveStream { stream_id: 2 })
            .expect("remove stream failed");
        tokio::time::sleep(Duration::from_millis(200)).await;

        command_tx
            .send(WorkerCommand::BindStream {
                stream_id: 2,
                request: crate::model::SourceBindingRequest {
                    namespace: namespace.clone(),
                    plane: PlaneKind::View,
                    path_expression: "odometry".to_string(),
                },
            })
            .expect("rebind command failed");
        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::SourceBound { stream_id: 2, .. })
        })
        .await
        .expect("rebind did not emit SourceBound");

        let replay = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 2,
                    records
                } if !records.is_empty()
            )
        })
        .await;
        assert!(replay.is_some(), "expected history replay after rebind");

        shutdown_worker_task(command_tx, cancel, worker_task).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn integration_ingest_toggle_pauses_and_resumes_updates() {
        let namespace = unique_namespace("viewer-ingest-toggle");
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let config = ViewerConfig {
            namespace: namespace.clone(),
            source_expression: "odometry".to_string(),
            storage_path: Some(session_storage_path()),
            ..ViewerConfig::default()
        };
        let worker_task = tokio::spawn(run_worker(config, command_rx, event_tx, cancel.clone()));

        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::Ready)
        })
        .await
        .expect("worker did not emit Ready");

        command_tx
            .send(WorkerCommand::BindStream {
                stream_id: 4,
                request: crate::model::SourceBindingRequest {
                    namespace: namespace.clone(),
                    plane: PlaneKind::View,
                    path_expression: "odometry".to_string(),
                },
            })
            .expect("bind command failed");
        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::SourceBound { stream_id: 4, .. })
        })
        .await
        .expect("source was not bound");

        let publisher_session = Session::create(&namespace)
            .await
            .expect("publisher session create failed");
        let publisher_node = publisher_session
            .create_node("viewer-ingest-toggle-publisher")
            .build()
            .await
            .expect("publisher node build failed");
        let publisher = publisher_node
            .advertise::<TestOdometry>("odometry")
            .build()
            .await
            .expect("publisher build failed");

        publisher
            .put(
                &TestOdometry {
                    x: 1.0,
                    y: 1.0,
                    theta: 1.0,
                },
                &publisher_session.now(),
            )
            .await
            .expect("baseline publish failed");
        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 4,
                    records
                } if !records.is_empty()
            )
        })
        .await
        .expect("expected baseline live update");

        command_tx
            .send(WorkerCommand::SetIngestEnabled(false))
            .expect("disable ingest command failed");
        tokio::time::sleep(Duration::from_millis(300)).await;

        for i in 0..3 {
            publisher
                .put(
                    &TestOdometry {
                        x: 10.0 + i as f64,
                        y: 10.0 + i as f64,
                        theta: 10.0 + i as f64,
                    },
                    &publisher_session.now(),
                )
                .await
                .expect("publish while disabled failed");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        let paused_records =
            recv_event_matching(&mut event_rx, Duration::from_millis(900), |event| {
                matches!(
                    event,
                    WorkerEvent::RecordsAppended {
                        stream_id: 4,
                        records
                    } if !records.is_empty()
                )
            })
            .await;
        assert!(
            paused_records.is_none(),
            "received live updates while ingest was disabled"
        );

        command_tx
            .send(WorkerCommand::SetIngestEnabled(true))
            .expect("enable ingest command failed");
        tokio::time::sleep(Duration::from_millis(250)).await;

        for i in 0..3 {
            publisher
                .put(
                    &TestOdometry {
                        x: 20.0 + i as f64,
                        y: 20.0 + i as f64,
                        theta: 20.0 + i as f64,
                    },
                    &publisher_session.now(),
                )
                .await
                .expect("publish after resume failed");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }

        let resumed_records = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 4,
                    records
                } if !records.is_empty()
            )
        })
        .await;
        assert!(
            resumed_records.is_some(),
            "expected live updates after ingest resume"
        );

        shutdown_worker_task(command_tx, cancel, worker_task).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "manual soak run; executes a longer live ingest loop"]
    async fn soak_worker_stays_healthy_under_continuous_stream() {
        let namespace = unique_namespace("viewer-soak");
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        let config = ViewerConfig {
            namespace: namespace.clone(),
            source_expression: "odometry".to_string(),
            storage_path: Some(session_storage_path()),
            ..ViewerConfig::default()
        };
        let worker_task = tokio::spawn(run_worker(config, command_rx, event_tx, cancel.clone()));

        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::Ready)
        })
        .await
        .expect("worker did not emit Ready");

        command_tx
            .send(WorkerCommand::BindStream {
                stream_id: 7,
                request: crate::model::SourceBindingRequest {
                    namespace: namespace.clone(),
                    plane: PlaneKind::View,
                    path_expression: "odometry".to_string(),
                },
            })
            .expect("bind command failed");
        let _ = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(event, WorkerEvent::SourceBound { stream_id: 7, .. })
        })
        .await
        .expect("source was not bound");

        let publisher_session = Session::create(&namespace)
            .await
            .expect("publisher session create failed");
        let publisher_node = publisher_session
            .create_node("viewer-soak-publisher")
            .build()
            .await
            .expect("publisher node build failed");
        let publisher = publisher_node
            .advertise::<TestOdometry>("odometry")
            .build()
            .await
            .expect("publisher build failed");

        let start = tokio::time::Instant::now();
        let mut sent = 0_u64;
        while start.elapsed() < Duration::from_secs(15) {
            publisher
                .put(
                    &TestOdometry {
                        x: sent as f64,
                        y: sent as f64,
                        theta: sent as f64,
                    },
                    &publisher_session.now(),
                )
                .await
                .expect("publish failed");
            sent = sent.saturating_add(1);
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        let records_event = recv_event_matching(&mut event_rx, Duration::from_secs(6), |event| {
            matches!(
                event,
                WorkerEvent::RecordsAppended {
                    stream_id: 7,
                    records
                } if !records.is_empty()
            )
        })
        .await;
        assert!(records_event.is_some(), "expected live records during soak");
        assert!(sent > 100, "soak sent too few samples: {sent}");

        shutdown_worker_task(command_tx, cancel, worker_task).await;
    }
}
