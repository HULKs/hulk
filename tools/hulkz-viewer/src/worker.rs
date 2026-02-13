use std::collections::BTreeMap;

mod commands;
mod discovery;
mod encoding;
mod lifecycle;
mod parameters;
mod streams;

use color_eyre::{eyre::WrapErr as _, Result};
use hulkz::Session;
use hulkz_stream::{OpenMode, StreamBackendBuilder, StreamRecord};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, warn};

use crate::model::{DisplayedRecord, StreamId, ViewerConfig, WorkerCommand, WorkerEvent};
use commands::{send_error, send_event};
use discovery::{
    emit_discovery_snapshot, emit_discovery_snapshot_or_error, insert_discovered_entity,
    reconcile_discovery_snapshot, remove_discovered_entity, restart_discovery_watchers,
    stop_discovery_watchers, DiscoveryEvent, DiscoveryState,
};
use encoding::stream_record_to_displayed_record;
use lifecycle::{shutdown_worker, storage_path_for_config};
use parameters::{read_parameter_value, write_parameter_value};
use streams::{
    apply_scrub_anchor, bind_source, emit_history_snapshot, resolve_record_at_anchor,
    spawn_live_updates_task, stop_stream_context, WorkerStreamContext,
};

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
    let mut pending_live_batches: BTreeMap<StreamId, Vec<DisplayedRecord>> = BTreeMap::new();
    let mut next_generation: u64 = 1;

    send_event(&event_tx, WorkerEvent::Ready)?;
    info!("worker ready; awaiting stream bind commands");

    let mut stats_interval = tokio::time::interval(config.poll_interval);
    stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut discovery_reconcile_interval =
        tokio::time::interval(config.discovery_reconcile_interval);
    discovery_reconcile_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut live_batch_flush_interval = tokio::time::interval(config.live_event_batch_delay);
    live_batch_flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
                                    flush_stream_batch(stream_id, &mut pending_live_batches, &event_tx)?;
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
                                let _ = send_event(
                                    &event_tx,
                                    WorkerEvent::SourceBound {
                                        stream_id,
                                        label: binding_label,
                                        binding,
                                    },
                                );
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
                        flush_stream_batch(stream_id, &mut pending_live_batches, &event_tx)?;
                        if let Some(existing) = streams.remove(&stream_id) {
                            info!(stream_id, "removing stream binding");
                            stop_stream_context(existing);
                        }
                    }
                    Some(WorkerCommand::ReadParameter(target)) => {
                        match read_parameter_value(&discovery_session, &target).await {
                            Ok(value_pretty) => {
                                let _ = send_event(
                                    &event_tx,
                                    WorkerEvent::ParameterValueLoaded { target, value_pretty },
                                );
                            }
                            Err(error) => {
                                send_error(&event_tx, format!("{error:#}"));
                            }
                        }
                    }
                    Some(WorkerCommand::SetParameter { target, value_json }) => {
                        match write_parameter_value(&discovery_session, &target, &value_json).await {
                            Ok(message) => {
                                let _ = send_event(
                                    &event_tx,
                                    WorkerEvent::ParameterWriteResult {
                                        target: target.clone(),
                                        success: true,
                                        message,
                                    },
                                );
                                match read_parameter_value(&discovery_session, &target).await {
                                    Ok(value_pretty) => {
                                        let _ = send_event(
                                            &event_tx,
                                            WorkerEvent::ParameterValueLoaded {
                                                target,
                                                value_pretty,
                                            },
                                        );
                                    }
                                    Err(error) => {
                                        send_error(&event_tx, format!("{error:#}"));
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = send_event(
                                    &event_tx,
                                    WorkerEvent::ParameterWriteResult {
                                        target,
                                        success: false,
                                        message: format!("{error:#}"),
                                    },
                                );
                            }
                        }
                    }
                    Some(WorkerCommand::SetScrubAnchor { stream_id, anchor_nanos }) => {
                        debug!(stream_id, anchor_nanos, "received scrub anchor");
                        if let Some(context) = streams.get(&stream_id) {
                            match resolve_record_at_anchor(&context.source, anchor_nanos).await {
                                Ok(record) => {
                                    let _ = send_event(
                                        &event_tx,
                                        WorkerEvent::AnchorRecord {
                                            stream_id,
                                            anchor_nanos,
                                            record,
                                        },
                                    );
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
                        let batch = pending_live_batches.entry(stream_id).or_default();
                        batch.push(stream_record_to_displayed_record(&record));
                        if batch.len() >= config.live_event_batch_max.max(1) {
                            flush_stream_batch(stream_id, &mut pending_live_batches, &event_tx)?;
                        }
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
                flush_all_stream_batches(&mut pending_live_batches, &event_tx)?;
                for (stream_id, context) in &streams {
                    send_event(&event_tx, WorkerEvent::StreamStats {
                            stream_id: *stream_id,
                            source: Box::new(context.source.stats_snapshot()),
                        })?;
                }
                send_event(&event_tx, WorkerEvent::BackendStats {
                    backend: Box::new(backend.stats_snapshot()),
                })?;
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
            _ = live_batch_flush_interval.tick() => {
                flush_all_stream_batches(&mut pending_live_batches, &event_tx)?;
            }
        }
    }

    flush_all_stream_batches(&mut pending_live_batches, &event_tx)?;
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

fn flush_stream_batch(
    stream_id: StreamId,
    pending_live_batches: &mut BTreeMap<StreamId, Vec<DisplayedRecord>>,
    event_tx: &UnboundedSender<WorkerEvent>,
) -> Result<()> {
    let Some(records) = pending_live_batches.remove(&stream_id) else {
        return Ok(());
    };
    if records.is_empty() {
        return Ok(());
    }
    send_event(
        event_tx,
        WorkerEvent::RecordsAppended { stream_id, records },
    )?;
    Ok(())
}

fn flush_all_stream_batches(
    pending_live_batches: &mut BTreeMap<StreamId, Vec<DisplayedRecord>>,
    event_tx: &UnboundedSender<WorkerEvent>,
) -> Result<()> {
    let stream_ids = pending_live_batches.keys().copied().collect::<Vec<_>>();
    for stream_id in stream_ids {
        flush_stream_batch(stream_id, pending_live_batches, event_tx)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
