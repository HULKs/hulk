use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::{eyre::eyre, eyre::WrapErr as _, Result};
use hulkz_stream::StreamBackend;
use tokio::sync::mpsc::Sender;

use crate::protocol::{ViewerConfig, WorkerEventEnvelope};

pub(super) fn session_storage_path() -> PathBuf {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("hulkz-viewer-{}-{run_id}", std::process::id()))
}

pub(super) fn storage_path_for_config(config: &ViewerConfig) -> PathBuf {
    config
        .storage_path
        .clone()
        .unwrap_or_else(session_storage_path)
}

pub(super) async fn shutdown_worker(
    backend: StreamBackend,
    driver_task: &mut tokio::task::JoinHandle<hulkz_stream::Result<()>>,
    event_tx: &Sender<WorkerEventEnvelope>,
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
