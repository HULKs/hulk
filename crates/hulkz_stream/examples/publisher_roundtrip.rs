use std::{
    io,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hulkz::{Scope, ScopedPath, Session};
use hulkz_stream::{
    NamespaceBinding, OpenMode, PlaneKind, Result, SourceHandle, SourceSpec, StreamBackendBuilder,
    StreamRecord,
};
use tokio::sync::broadcast::{self, error::RecvError};

const SOURCE_NAMESPACE: &str = "demo";
const SOURCE_PATH: &str = "odometry";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Waiting for publisher on namespace='{SOURCE_NAMESPACE}' path='{SOURCE_PATH}'");
    println!("Run in another terminal: cargo run -p hulkz --example publisher");

    let session = Session::create("hulkz-stream-roundtrip").await?;
    let storage_root = unique_storage_root();

    let (backend, driver) = StreamBackendBuilder::new(session)
        .open_mode(OpenMode::ReadWrite)
        .storage_path(storage_root)
        .build()
        .await?;
    let driver_task = tokio::spawn(driver);

    let source = backend
        .source(SourceSpec {
            plane: PlaneKind::View,
            path: ScopedPath::new(Scope::Local, SOURCE_PATH),
            node_override: None,
            namespace_binding: NamespaceBinding::Pinned(SOURCE_NAMESPACE.to_string()),
        })
        .await?;
    let mut live_updates = source.live_updates();

    let verify_result = verify_roundtrip(&source, &mut live_updates).await;

    let shutdown_result = async {
        backend.shutdown().await?;
        driver_task.await??;
        Result::<()>::Ok(())
    }
    .await;

    verify_result?;
    shutdown_result?;
    Ok(())
}

async fn verify_roundtrip(
    source: &SourceHandle,
    live_updates: &mut broadcast::Receiver<StreamRecord>,
) -> Result<()> {
    let first = wait_for_next_record(live_updates, Duration::from_secs(10)).await?;
    let second = wait_for_next_record(live_updates, Duration::from_secs(5)).await?;

    let before = source
        .before_or_equal(second.timestamp)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "before_or_equal returned None"))?;
    let range = source
        .range_inclusive(first.timestamp, second.timestamp)
        .await?;

    if range.is_empty() {
        return Err(
            io::Error::new(io::ErrorKind::NotFound, "range_inclusive returned empty").into(),
        );
    }

    println!(
        "Roundtrip OK: first={}ns second={}ns before={}ns range_len={} payload={}B",
        first.timestamp.get_time().as_nanos(),
        second.timestamp.get_time().as_nanos(),
        before.timestamp.get_time().as_nanos(),
        range.len(),
        second.payload.len(),
    );

    Ok(())
}

async fn wait_for_next_record(
    live_updates: &mut broadcast::Receiver<StreamRecord>,
    timeout_after: Duration,
) -> Result<StreamRecord> {
    let started = tokio::time::Instant::now();

    loop {
        let remaining = timeout_after.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "timeout waiting for new record on {SOURCE_NAMESPACE}/{SOURCE_PATH} after {:?}",
                    timeout_after
                ),
            )
            .into());
        }

        match tokio::time::timeout(remaining, live_updates.recv()).await {
            Ok(Ok(record)) => return Ok(record),
            Ok(Err(RecvError::Lagged(_))) => continue,
            Ok(Err(RecvError::Closed)) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "live update channel closed",
                )
                .into())
            }
            Err(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    format!(
                    "timeout waiting for new record on {SOURCE_NAMESPACE}/{SOURCE_PATH} after {:?}",
                    timeout_after
                ),
                )
                .into())
            }
        }
    }
}

fn unique_storage_root() -> PathBuf {
    let epoch_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hulkz-stream-roundtrip-{}-{epoch_nanos}",
        std::process::id()
    ))
}
