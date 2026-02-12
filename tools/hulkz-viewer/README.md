# hulkz-viewer

`hulkz-viewer` is an MVP egui visualization tool for `hulkz-stream`.

Requirements and roadmap live in `tools/hulkz-viewer/USER_STORIES_AND_REQUIREMENTS.md`.

Current MVP behavior:

- starts with one default `Text` panel: path `odometry`, view plane, namespace follows global default
- supports multiple `Text` panels with independent source bindings (`namespace`, `path` DSL)
- each `Text` panel rebind restores recorded session history for that source before continuing live updates
- uses `egui-dock` tabs (`Discovery`, `Timeline`, `Text`, `Parameters`)
- global default namespace starts unset and can be edited inline; discovery is namespace-scoped
- each `Text` panel chooses namespace mode: follow global default or explicit override
- `Text` panels always bind to the `View` plane
- one dedicated `Timeline` panel owns global scrub controls (`Prev`/`Next`/`Jump Latest` + slider) and drives all `Text` panels
- shows discovered publishers/parameters from `hulkz` graph (watch-based, namespace-scoped, with periodic reconciliation)
- parameter tab supports select + refresh + staged JSON apply with explicit `Apply`
- persists dock layout and core UI settings between runs
- supports CLI startup overrides for namespace/source/storage path
- each `Text` panel shows only the value at the current global timeline anchor
- text value resolution uses `before_or_equal(anchor)` per source (no per-panel record-list state)
- renders live JSON payloads
- supports synchronized stepping/scrubbing across text panels
- uses session-only temporary storage for in-memory + short history scrub
- uses push-based live ingest updates from `hulkz-stream` (no query polling loop)
- source path input uses completion dropdown suggestions from discovery
- includes async worker integration tests and a manual soak test harness

## Run

Terminal A (publisher):

```bash
cargo run -p hulkz --example publisher
```

Terminal B (viewer):

```bash
cargo run -p hulkz-viewer
```

Override startup settings:

```bash
cargo run -p hulkz-viewer -- \
  --namespace demo \
  --source odometry \
  --storage-path /tmp/hulkz-viewer-history
```

To debug ingest behavior, enable logs:

```bash
RUST_LOG=hulkz_viewer=trace,hulkz_stream=debug cargo run -p hulkz-viewer
```

Run viewer test suite:

```bash
cargo test -p hulkz-viewer
```

Optional manual soak harness:

```bash
cargo test -p hulkz-viewer soak_worker_stays_healthy_under_continuous_stream -- --ignored
```

UI soak checklist/runbook:

- `tools/hulkz-viewer/SOAK_RUNBOOK.md`

## Notes

- Path DSL examples in UI:
  - `odometry` (local)
  - `/fleet/topic` (global)
  - `~node/private_topic` (private + explicit node override)
- Discovery panel uses right-click context actions to open a new `Text` panel.
- The viewer sends scrub hints to `hulkz-stream` via scrub-window + prefetch.
