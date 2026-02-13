use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz::{ParameterInfo, PublisherInfo, Scope, ScopedPath, SessionInfo};
use hulkz_stream::{NamespaceBinding, SourceHandle, SourceSpec, StreamBackend, StreamRecord};
use tokio::sync::{broadcast::error::RecvError, mpsc::Sender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::protocol::{
    DiscoveredParameter, DiscoveredPublisher, DiscoveredSession, DisplayedRecord,
    RecordChunkSource, SourceBindingInfo, SourceBindingRequest, StreamId, ViewerConfig,
    WorkerEvent, WorkerEventEnvelope,
};

use super::{
    commands::send_event,
    encoding::{stream_record_to_displayed_record, timestamp_from_nanos},
    WorkerInternalEvent,
};

pub(super) struct WorkerStreamContext {
    pub(super) source: SourceHandle,
    pub(super) generation: u64,
    pub(super) live_cancel: CancellationToken,
    pub(super) live_task: tokio::task::JoinHandle<()>,
}

pub(super) async fn emit_history_snapshot(
    stream_id: StreamId,
    generation: u64,
    source: &SourceHandle,
    event_tx: &Sender<WorkerEventEnvelope>,
) -> Result<()> {
    let stats = source.stats_snapshot();
    let (Some(start), Some(end)) = (stats.durable_oldest, stats.durable_latest) else {
        debug!("no durable history available for source");
        send_event(
            event_tx,
            WorkerEvent::StreamHistoryEnd {
                stream_id,
                generation,
                total_records: 0,
            },
        )
        .await?;
        return Ok(());
    };

    send_event(
        event_tx,
        WorkerEvent::StreamHistoryBegin {
            stream_id,
            generation,
        },
    )
    .await?;

    let mut range_stream = source
        .range_inclusive_stream(start, end, 1024)
        .await
        .wrap_err("failed to query durable source history")?;

    let mut total_records = 0_usize;
    while let Some(chunk_result) = range_stream.recv().await {
        let chunk = chunk_result?;
        if chunk.is_last {
            break;
        }
        if chunk.records.is_empty() {
            continue;
        }

        let records = chunk
            .records
            .iter()
            .map(stream_record_to_displayed_record)
            .collect::<Vec<_>>();
        total_records = total_records.saturating_add(records.len());
        send_event(
            event_tx,
            WorkerEvent::StreamRecordsChunk {
                stream_id,
                generation,
                records,
                source: RecordChunkSource::History,
            },
        )
        .await?;
    }
    info!(
        stream_id,
        generation, total_records, "emitted source history"
    );
    send_event(
        event_tx,
        WorkerEvent::StreamHistoryEnd {
            stream_id,
            generation,
            total_records,
        },
    )
    .await?;
    Ok(())
}

pub(super) fn to_discovered_publisher(info: PublisherInfo) -> DiscoveredPublisher {
    let path_expression = scoped_path_expression(info.scope, &info.path, Some(&info.node));
    DiscoveredPublisher {
        namespace: info.namespace,
        node: info.node,
        path_expression,
    }
}

pub(super) fn to_discovered_parameter(info: ParameterInfo) -> DiscoveredParameter {
    let path_expression = scoped_path_expression(info.scope, &info.path, Some(&info.node));
    DiscoveredParameter {
        namespace: info.namespace,
        node: info.node,
        path_expression,
    }
}

pub(super) fn to_discovered_session(info: SessionInfo) -> DiscoveredSession {
    DiscoveredSession {
        namespace: info.namespace,
        id: info.id,
    }
}

pub(super) async fn apply_scrub_anchor(
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

pub(super) async fn resolve_record_at_anchor(
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

pub(super) async fn bind_source(
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

pub(super) fn parse_source_path_expression(input: &str) -> Result<(ScopedPath, Option<String>)> {
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

pub(super) fn scoped_path_expression(
    scope: Scope,
    path: &str,
    private_node: Option<&str>,
) -> String {
    match scope {
        Scope::Global => format!("/{path}"),
        Scope::Local => path.to_string(),
        Scope::Private => match private_node {
            Some(node) => format!("~{node}/{path}"),
            None => format!("~/{path}"),
        },
    }
}

pub(super) fn spawn_live_updates_task(
    stream_id: StreamId,
    generation: u64,
    mut live_updates: tokio::sync::broadcast::Receiver<StreamRecord>,
    internal_event_tx: Sender<WorkerInternalEvent>,
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
                                .await
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
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(RecvError::Closed) => {
                            let _ = internal_event_tx
                                .send(WorkerInternalEvent::Closed {
                                    stream_id,
                                    generation,
                                })
                                .await;
                            break;
                        }
                    }
                }
            }
        }
    })
}

pub(super) fn stop_stream_context(context: WorkerStreamContext) {
    context.live_cancel.cancel();
    context.live_task.abort();
    drop(context.source);
}
