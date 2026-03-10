# hulkz-stream

`hulkz-stream` is an async stream backend for live + historical robotics data.

## What it provides

- Shared durable MCAP storage with rolling segments
- Deduplicated ingest per logical source identity
- Cache-first + indexed durable queries (`latest`, `before_or_equal`, `nearest`, `range_inclusive`)
- Source and backend stats (`watch` + snapshot)
- Push-based live ingest notifications (`SourceHandle::live_updates`)
- Timeline aggregation APIs for UI rendering
- Scrub helpers (`set_scrub_window`, `prefetch_range`, `prefetch_range_cancellable`)

## Runtime model

`StreamBackendBuilder::build()` returns `(StreamBackend, StreamDriver)`.

You spawn the driver explicitly:

```rust
let (backend, driver) = StreamBackendBuilder::new(session)
    .storage_path("/tmp/hulkz-stream")
    .build()
    .await?;

let driver_task = tokio::spawn(driver);

// use backend...

backend.shutdown().await?;
driver_task.await??;
```

## Logging

`hulkz-stream` emits structured logs via `tracing` but does not install a global subscriber.
Enable logs from your binary with `RUST_LOG`, for example:

```bash
RUST_LOG=hulkz_stream=debug cargo run -p hulkz-stream --example backend_driver
```

Live ingestion can be consumed without polling:

```rust
let source = backend.source(spec).await?;
let mut updates = source.live_updates();
while let Ok(record) = updates.recv().await {
    // render/update UI immediately
}
```

## Durability semantics

- Ingest records are written to the active MCAP segment.
- `durable_frontier` advances only after write + flush + file `sync_data` complete.
- `ingest_frontier` may lead `durable_frontier` under load.

## Backpressure policy

Writer queue policy is deterministic blocking backpressure:

- no silent record drops when the queue is full
- ingest workers wait for queue capacity
- backend stats expose queue depth, high-watermark, and backpressure event count

## Examples

- `examples/storage_roundtrip.rs`
- `examples/backend_driver.rs`
- `examples/publisher_roundtrip.rs`

Run examples:

```bash
cargo run -p hulkz-stream --example storage_roundtrip
cargo run -p hulkz-stream --example backend_driver
cargo run -p hulkz-stream --example publisher_roundtrip
```

`examples/publisher_roundtrip.rs` uses `PlaneKind::View` for `odometry` because this workspace
mirrors that feed onto the view plane. If view mirroring is disabled in your deployment, switch
the source to `PlaneKind::Data`.
