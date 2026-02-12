use std::{num::NonZeroU128, sync::Arc, time::Duration};

use hulkz::{Scope, ScopedPath};
use hulkz_stream::{
    storage::Storage, NamespaceBinding, OpenMode, PlaneKind, Result, SourceSpec, StreamRecord,
};

fn ts(nanos: u64) -> hulkz::Timestamp {
    let id: zenoh::time::TimestampId = NonZeroU128::new(1).expect("non-zero").into();
    hulkz::Timestamp::new(zenoh::time::NTP64::from(Duration::from_nanos(nanos)), id)
}

#[tokio::main]
async fn main() -> Result<()> {
    let root = std::env::temp_dir().join(format!(
        "hulkz-stream-storage-example-{}",
        std::process::id()
    ));

    let storage = Storage::open(OpenMode::ReadWrite, root.clone(), 1024 * 1024).await?;

    let source = SourceSpec {
        plane: PlaneKind::Data,
        path: ScopedPath::new(Scope::Local, "camera/front"),
        node_override: None,
        namespace_binding: NamespaceBinding::Pinned("robot".to_string()),
    };

    storage
        .append(StreamRecord {
            source: source.clone(),
            effective_namespace: Some("robot".to_string()),
            timestamp: ts(1_000),
            encoding: zenoh::bytes::Encoding::APPLICATION_CDR,
            payload: Arc::from([1_u8, 2, 3, 4]),
        })
        .await?;

    if let Some(latest) = storage.query_latest(&source).await? {
        println!(
            "latest={}ns payload={}B",
            latest.timestamp.get_time().as_nanos(),
            latest.payload.len()
        );
    }

    storage.shutdown().await?;
    Ok(())
}
