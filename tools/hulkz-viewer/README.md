# hulkz-viewer

`hulkz-viewer` is an MVP egui visualization tool for `hulkz-stream`.

Current MVP behavior:

- subscribes to one hardcoded source: namespace `demo`, path `odometry`, view plane
- renders live JSON payloads
- supports stepping (`Prev`/`Next`), jump-to-latest, and index scrub slider
- uses session-only temporary storage for in-memory + short history scrub
- uses push-based live ingest updates from `hulkz-stream` (no query polling loop)

## Run

Terminal A (publisher):

```bash
cargo run -p hulkz --example publisher
```

Terminal B (viewer):

```bash
cargo run -p hulkz-viewer
```

To debug ingest behavior, enable logs:

```bash
RUST_LOG=hulkz_viewer=trace,hulkz_stream=debug cargo run -p hulkz-viewer
```

## Notes

- For this MVP there are no CLI args or source pickers.
- The viewer sends scrub hints to `hulkz-stream` via scrub-window + prefetch.
