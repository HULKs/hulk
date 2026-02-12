use std::{
    num::NonZeroU128,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz::{Scope, ScopedPath, Session, Timestamp};
use hulkz_stream::{
    NamespaceBinding, OpenMode, PlaneKind, SourceHandle, SourceSpec, StreamBackend,
    StreamBackendBuilder, StreamRecord,
};
use tokio::sync::{
    broadcast::error::RecvError,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, trace, warn};

use crate::model::{RecordRow, ViewerConfig, WorkerCommand, WorkerEvent};

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

async fn run_worker_inner(
    config: ViewerConfig,
    mut command_rx: UnboundedReceiver<WorkerCommand>,
    event_tx: UnboundedSender<WorkerEvent>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    info!(
        namespace = config.namespace,
        source_path = config.source_path,
        "worker starting"
    );
    let session = Session::create(config.namespace).await.wrap_err_with(|| {
        format!(
            "failed to create hulkz session for namespace {}",
            config.namespace
        )
    })?;

    let storage_path = session_storage_path();
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

    let source_spec = SourceSpec {
        plane: PlaneKind::View,
        path: ScopedPath::new(Scope::Local, config.source_path),
        node_override: None,
        namespace_binding: NamespaceBinding::Pinned(config.namespace.to_string()),
    };

    let source = backend
        .source(source_spec.clone())
        .await
        .wrap_err_with(|| {
            format!(
                "failed to acquire source plane={:?} path={} namespace={}",
                source_spec.plane,
                source_spec.path.path(),
                config.namespace
            )
        })?;
    info!("source handle acquired");
    event_tx
        .send(WorkerEvent::Ready)
        .map_err(|_| eyre!("failed to send Ready event: worker event channel closed"))?;

    let mut live_updates = source.live_updates();
    info!("subscribed to source live updates");

    let mut stats_interval = tokio::time::interval(config.poll_interval);
    stats_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

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
                    Some(WorkerCommand::SetFollowLive(enabled)) => {
                        debug!(enabled, "received follow-live command");
                    }
                    Some(WorkerCommand::SetScrubAnchor(anchor_nanos)) => {
                        debug!(anchor_nanos, "received scrub anchor");
                        if let Err(error) = apply_scrub_anchor(&backend, &source, &config, anchor_nanos).await {
                            send_error(&event_tx, format!("{error:#}"));
                        }
                    }
                    Some(WorkerCommand::Shutdown) | None => {
                        info!("worker shutdown command received");
                        break;
                    }
                }
            }
            live = live_updates.recv() => {
                match live {
                    Ok(record) => {
                        trace!(
                            timestamp_nanos = record.timestamp.get_time().as_nanos(),
                            payload_bytes = record.payload.len(),
                            "live record received"
                        );
                        event_tx
                            .send(WorkerEvent::RecordsAppended(vec![record_to_row(&record)]))
                            .map_err(|_| eyre!("failed to send record event: worker event channel closed"))?;
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        warn!(skipped, "live updates receiver lagged");
                        send_error(&event_tx, format!("live updates lagged; skipped {skipped} records"));
                    }
                    Err(RecvError::Closed) => {
                        return Err(eyre!("live updates channel closed unexpectedly"));
                    }
                }
            }
            _ = stats_interval.tick() => {
                trace!("publishing stats snapshot");
                event_tx
                    .send(WorkerEvent::Stats {
                        source: Box::new(source.stats_snapshot()),
                        backend: Box::new(backend.stats_snapshot()),
                    })
                    .map_err(|_| eyre!("failed to send stats event: worker event channel closed"))?;
            }
        }
    }

    info!("worker shutting down");
    shutdown_worker(backend, source, &mut driver_task, &event_tx)
        .await
        .wrap_err("failed during worker shutdown")?;
    info!("worker stopped");
    Ok(())
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

async fn shutdown_worker(
    backend: StreamBackend,
    source: SourceHandle,
    driver_task: &mut tokio::task::JoinHandle<hulkz_stream::Result<()>>,
    event_tx: &UnboundedSender<WorkerEvent>,
) -> Result<()> {
    drop(source);

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

fn send_error(event_tx: &UnboundedSender<WorkerEvent>, message: String) {
    warn!(%message, "worker error");
    let _ = event_tx.send(WorkerEvent::Error(message));
}

fn session_storage_path() -> PathBuf {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("hulkz-viewer-{}-{run_id}", std::process::id()))
}

fn timestamp_from_nanos(nanos: u64) -> Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

fn to_nanos(timestamp: &Timestamp) -> u64 {
    timestamp.get_time().as_nanos()
}

fn record_to_row(record: &StreamRecord) -> RecordRow {
    let encoding = record.encoding.to_string();
    let (json_pretty, raw_fallback) = decode_payload(&encoding, &record.payload);

    RecordRow {
        timestamp_nanos: to_nanos(&record.timestamp),
        effective_namespace: record.effective_namespace.clone(),
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
    use super::decode_payload;

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
}
