use std::{num::NonZeroU128, time::Duration};

use hulkz::{Scope, ScopedPath, Session};
use hulkz_stream::{
    NamespaceBinding, OpenMode, PlaneKind, Result, SourceSpec, StreamBackendBuilder,
};

fn ts(nanos: u64) -> hulkz::Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    hulkz::Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::create("hulkz-stream-example").await?;

    let root = std::env::temp_dir().join(format!(
        "hulkz-stream-backend-example-{}",
        std::process::id()
    ));
    let (backend, driver) = StreamBackendBuilder::new(session)
        .open_mode(OpenMode::ReadWrite)
        .storage_path(root)
        .cache_budget_bytes(64 * 1024 * 1024)
        .build()
        .await?;

    let driver_task = tokio::spawn(driver);

    let source = backend
        .source(SourceSpec {
            plane: PlaneKind::Data,
            path: ScopedPath::new(Scope::Local, "camera/front"),
            node_override: None,
            namespace_binding: NamespaceBinding::FollowTarget,
        })
        .await?;

    let start = ts(0);
    let end = ts(10_000_000_000);

    backend.set_scrub_window(Some((start, end))).await?;
    let _prefetched = source.prefetch_range(start, end).await?;

    backend.shutdown().await?;
    driver_task.await??;
    Ok(())
}
