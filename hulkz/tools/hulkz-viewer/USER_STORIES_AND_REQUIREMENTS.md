# hulkz-viewer: User Stories and Requirements

## 1. Purpose

`hulkz-viewer` is an egui desktop tool for interactive robotics stream inspection.

It must support:

- live visualization of `hulkz` streams,
- low-friction source discovery and subscription,
- stable step/scrub through in-session history,
- safe parameter inspection and writes,
- responsive operation while ingest and disk persistence are active.

## 2. Product Principles

1. UI must stay responsive; async/data work runs off the UI thread.
2. Viewer behavior is source-accurate; no hidden plane fallback.
3. Live ingest and historical scrub are both first-class workflows.
4. Session-only mode is default; persistent mode is optional.
5. Discovery and visualization should match `hulkz` namespace/scope/plane semantics.
6. Errors are visible and actionable, not silent.
7. Logging should describe runtime control flow and API calls, not noisy builder internals.

## 3. Scope and Non-Scope

### In scope (v2 plan target)

- Dedicated crate: `tools/hulkz-viewer`.
- Hybrid-shell egui app:
  - fixed/collapsible `Discovery` pane (left),
  - fixed/collapsible `Timeline` pane (bottom),
  - `egui-dock` workspace for `Text` and `Parameters`.
- Initial panel set:
  - source discovery,
  - global timeline scrub,
  - text/json stream view,
  - parameter panel.
- Multi-source subscription with panel-local source binding.
- Global default namespace used by discovery and by panels that follow default namespace.
- One global timeline stepping/scrub control synchronized across text panels.
- Runtime ingest control and follow-live mode.
- Structured logging and color-eyre error reporting.

### Out of scope (this iteration)

- Full plotting panel suite (2D/3D plots, images, transforms).
- Fleet-wide namespace aggregation in first pass.
- Advanced query DSL beyond stream/time primitives from `hulkz-stream`.
- Auth/permissions model for parameter writes.

## 4. Locked Decisions

1. Architecture: multi-source V2 using a hybrid shell + workspace dock.
2. Initial panel set: Discovery + Timeline + Text + Parameters.
3. Discovery scope: current namespace first.
4. Plane behavior: `Text` panels bind to `View` plane in this iteration.
5. Config strategy: mixed persisted layout + UI settings + CLI args.
6. History mode: session-only temp storage by default, optional persistent path.
7. Parameter writes: explicit `Apply` with clear success/error confirmation.
8. Runtime model: non-blocking UI with async worker(s).

## 5. Component Model

1. App Shell

- Owns egui frame lifecycle, dock layout, and panel routing.

2. Worker Runtime

- Owns async runtime integration, `hulkz` session, `hulkz-stream` backend/driver, and source handles.

3. Discovery Model

- Tracks discoverable sources (first pass: current namespace) and subscription state.

4. History Model

- Tracks selected source timeline cursor, follow-live mode, and scrub hints.

5. Parameter Model

- Tracks parameter tree, staged edits, apply actions, and result state.

6. Telemetry/Health Model

- Tracks backend/source stats, lag/frontier state, queue pressure, and operational errors.

## 6. Core Definitions

- ShellPane: fixed app chrome pane (`Controls`, `Discovery`, `Timeline`, `Status`).
- WorkspacePanel: dockable/openable workspace tab (`Text`, `Parameters`).
- Source Descriptor: `namespace + scope/path + optional node` identity used by viewer.
- Active Binding: mapping from panel to one selected source descriptor.
- Follow Live: global timeline mode that auto-advances to newest known timestamp.
- Scrub Anchor: global user-selected timestamp/index around which viewer requests hot window/prefetch for all active stream bindings.
- Session Storage: temporary `hulkz-stream` storage path tied to app run lifecycle.

## 7. User Stories and Acceptance Criteria

### US-001 Discover Sources in Current Namespace

As an operator, I want to see available sources in the current namespace so I can bind panels quickly.

Acceptance criteria:

1. Discovery panel lists sources from current namespace.
2. Source entries expose plane/scope/path identity clearly.
3. Selecting a source binds it to a chosen panel without blocking UI.
4. Discovery provides context actions to open a new text panel bound to `View`.

### US-002 Inspect Live JSON/Text Payloads

As a developer, I want to view the value at the current global timeline anchor in text form so I can debug payload semantics.

Acceptance criteria:

1. Text panel shows the selected record timestamp, effective namespace, and payload body for the current global anchor.
2. JSON payloads are pretty-printed when possible.
3. Non-JSON payloads show safe fallback text/hex preview.

### US-003 Step and Scrub History

As a debugger, I want to move backward/forward through records while live ingest continues.

Acceptance criteria:

1. Timeline panel provides global `Prev`, `Next`, `Jump Latest`, and canvas scrub controls.
2. When global `Follow live` is off, new records do not disturb the selected global anchor.
3. Scrub triggers window/prefetch hints through `hulkz-stream` APIs for each bound text panel.

### US-004 Toggle Ingest During Inspection

As an operator, I want to pause ingest while inspecting old records and resume afterward.

Acceptance criteria:

1. Ingest toggle maps to backend ingest enable/disable control.
2. UI remains responsive while toggling.
3. State and errors are visible in status/notifications.

### US-005 Multi-Source Workspace

As an analyst, I want multiple panels bound to different sources so I can compare streams.

Acceptance criteria:

1. User can open multiple text panels.
2. Each panel maintains independent source binding and cursor state.
3. Dock layout can be rearranged and restored from saved state.

### US-006 Safe Parameter Writes

As a controls engineer, I want explicit apply confirmation for parameter updates.

Acceptance criteria:

1. Parameter edits are staged before apply.
2. Apply action is explicit and surfaces success/failure.
3. Failed writes preserve user input and show error details.

### US-007 Robust Runtime Visibility

As a maintainer, I want useful logs and stats so failures can be diagnosed quickly.

Acceptance criteria:

1. Viewer emits structured tracing for key control-flow steps.
2. Source/backend stats are visible in UI and update over time.
3. Errors include context using `color-eyre` (`wrap_err` / `wrap_err_with`).

## 8. Functional Requirements

### FR-A App and Runtime Lifecycle

- FR-A01: Viewer starts with a working async runtime and does not block egui thread.
- FR-A02: Worker runtime initializes `hulkz` session, `hulkz-stream` backend, and explicit stream driver.
- FR-A03: App shutdown requests worker shutdown, cancels background work, and joins with timeout.

### FR-B Discovery and Source Binding

- FR-B01: Discovery lists sources for the selected global default namespace.
- FR-B01a: If global default namespace is unset, discovery stays empty/disabled until a namespace is entered.
- FR-B02: Source identity shown in discovery must include scoped path and namespace.
- FR-B03: `Text` panel bindings use `View` plane for this iteration.
- FR-B04: Opening/binding a source panel creates or reuses source handle via backend APIs.
- FR-B05: Source path input accepts hulkz-style DSL (`local`, `/global`, `~node/private`).

### FR-C Record Ingest and Display

- FR-C01: Record updates for active bindings are push-driven from `hulkz-stream` live updates.
- FR-C02: Text panel supports JSON pretty-print and safe fallback display.
- FR-C03: Decode failures are non-fatal and represented in UI.
- FR-C04: Text panel selection is deterministic and derived from the current global anchor (`<= anchor` nearest record).
- FR-C05: Text panel value resolution at scrub anchor uses source `before_or_equal(anchor)` queries instead of panel-local record list traversal.

### FR-D History and Scrub

- FR-D01: Timeline panel supports global latest, step-prev, step-next, and jump-latest interactions.
- FR-D02: Scrub emits bounded/debounced anchor updates.
- FR-D03: Scrub anchors update scrub working window and prefetch hints.
- FR-D04: Global follow-live mode can be toggled without losing per-panel record history.
- FR-D05: Timeline supports pointer-centered zoom + drag-pan over a clamped viewport range.
- FR-D06: Manual time interactions (scrub/pan/zoom) immediately disable follow-live until `Jump Latest` or explicit re-enable; lane-scroll does not disable follow-live.
- FR-D07: Timeline y-axis is dynamic stream/topic lanes (virtualized window over many lanes), sorted lexicographically by canonical path then namespace.
- FR-D08: Wheel interaction mapping is fixed: `Wheel = lane scroll`, `Shift+Wheel = time pan`, `Ctrl+Wheel = time zoom`.
- FR-D09: Timeline markers use lane-local diamonds: one sample marker per visible sample at fine zoom; clustered stretched diamonds at coarse zoom using per-lane bucketing.
- FR-D10: After follow-live is disabled, a zoomed/manual time window remains fixed in absolute time; if older history is trimmed, viewport clamps to nearest retained range without auto re-enabling follow-live.
- FR-D11: Lane set includes active bindings and lanes that have produced samples in-session; unbound lanes with history can remain visible.

### FR-E Multi-Panel Workspace

- FR-E01: Viewer uses a hybrid shell with fixed/collapsible Discovery/Timeline panes and `egui-dock` for workspace tab composition (`Text`, `Parameters`).
- FR-E02: Multiple text panels can coexist with independent bindings while sharing one global timeline anchor controlled by the Timeline panel.
- FR-E02a: Each text panel has explicit namespace mode: follow global default or override namespace.
- FR-E03: Workspace dock layout, shell visibility toggles, and panel-local settings can be persisted and restored.
- FR-E04: Panel draw code uses `PanelContext` + `UiIntent`; panel modules emit intents and app-side command dispatch applies mutations centrally.

### FR-F Parameters

- FR-F01: Parameter panel reads and displays parameters for selected node/source context.
- FR-F02: Parameter updates require explicit apply action.
- FR-F03: Apply results are reported as success/error with context.

### FR-G Configuration and Persistence

- FR-G01: CLI args can override namespace and storage mode/path.
- FR-G02: UI settings, shell visibility toggles, and workspace dock layout persist between runs.
- FR-G03: Default mode uses session-only temporary storage path.
- FR-G04: Optional persistent mode uses user-provided storage path.

### FR-H Observability and Error Handling

- FR-H01: Viewer initializes `tracing` subscriber with `RUST_LOG` support.
- FR-H02: Control-flow operations are logged (session open, backend build, source bind, shutdown, failures).
- FR-H03: Errors propagate with context via `color-eyre`.
- FR-H04: UI exposes last error and basic source/backend stats.

### FR-I Performance and Responsiveness

- FR-I01: UI frame loop must remain responsive under steady ingest.
- FR-I02: Heavy operations (open/bind/query/decode) run off UI thread.
- FR-I03: Scrub interactions avoid pathological query storms via debounce/rate limiting.
- FR-I04: Worker->UI event ingestion is budgeted per frame by wall-time, event count, and byte budget.
- FR-I05: Worker->UI event transport is bounded; backpressure prevents unbounded memory growth.
- FR-I06: Repaint is event-driven with short-delay wakeups under activity; no unconditional idle polling loop.
- FR-I07: UI->worker command transport is bounded; pending commands are buffered and flushed incrementally from the UI frame loop.

## 9. Non-Functional Requirements

- NFR-001: Viewer should remain interactive at typical developer ingest rates.
- NFR-002: Startup and source binding failures are diagnosable from logs alone.
- NFR-003: Memory growth is bounded by backend cache settings and panel state limits.
- NFR-004: Behavior is deterministic across reconnects and source rebinding.

## 10. Roadmap / Delivery Phases

1. Phase 1: Requirements lock + app shell baseline.
2. Phase 2: Introduce `egui-dock` and panel registry.
3. Phase 3: Discovery panel with current-namespace source listing + binding actions.
4. Phase 4: Text panel generalized to panel-local source binding and cursor state.
5. Phase 5: Parameter panel with read + staged apply.
6. Phase 6: Config persistence (layout/settings) + CLI override pass.
7. Phase 7: Hardening (error surfacing, logging polish, shutdown robustness, soak tests).

## 11. Test and Validation Matrix

### Unit tests

1. Selection behavior with follow-live on/off.
2. Step controls clamp behavior.
3. Decode behavior (JSON pretty, malformed fallback, binary fallback).
4. Scrub debounce/rate-limit behavior.
5. Panel binding model state transitions.

### Integration tests

1. Worker startup path creates session/backend/driver and emits ready event.
2. Source bind receives live updates from a real in-process publisher session.
3. Source rebind replays durable history snapshot for the bound source.
4. Discovery namespace snapshot includes active sessions for the namespace.
5. Ingest toggle pauses/resumes updates without handle invalidation.
6. Multi-panel independent cursor and binding behavior.
7. Parameter apply success/failure propagation.

### Manual scenarios

1. Terminal A: `cargo run -p hulkz --example publisher`.
2. Terminal B: `cargo run -p hulkz-viewer`.
3. Validate discovery list, bind source, inspect live records.
4. Scrub backward/forward while ingest continues.
5. Toggle ingest and verify stop/resume behavior.
6. Close viewer during ingest and verify clean shutdown.

### CI gates

1. `cargo check -p hulkz-viewer`
2. `cargo clippy -p hulkz-viewer --all-targets -- -D warnings`
3. `cargo test -p hulkz-viewer`
4. Optional manual soak harness: `cargo test -p hulkz-viewer soak_worker_stays_healthy_under_continuous_stream -- --ignored`
5. Import hygiene check: `rg -n "super::super" tools/hulkz-viewer/src/app tools/hulkz-viewer/src/worker`

## 12. Assumptions and Defaults

1. Default text panel source for smoke path remains local path `odometry` on `View`; namespace follows global default.
2. Public time type remains `hulkz::Timestamp`; viewer may derive nanos only for local UI indexing.
3. `hulkz-stream` remains backend of record for ingest/history/scrub semantics.
4. First discovery pass is namespace-local; broader discovery can be added later.
5. Requirements file is the source of truth for viewer scope/behavior during v2 delivery.

## 13. Implementation Status

- Completed: Phase 1 requirements lock and app shell baseline.
- Completed: Phase 2 shell/workspace layout with fixed `Discovery` + `Timeline` panes and `egui-dock` workspace tabs (`Text`, `Parameters`).
- Completed: Phase 3 namespace-scoped source discovery with bind actions.
- Completed: Phase 4 text panel generalized to multiple text tabs with panel-local bindings and one global synchronized timeline control (text panels show value at anchor only).
- Completed: Phase 5 parameter workflow core (read + staged JSON apply + explicit `Apply` + result feedback).
- Completed: global default namespace control for discovery/new panels plus discovery context-menu open actions for `Text`.
- Completed: per-text-panel namespace mode (`FollowDefault` or explicit override), removing sibling bool/string override state.
- Completed: Phase 6 persistence/overrides (dock layout + core UI settings persistence, plus CLI overrides for namespace/source/storage path).
- Completed: Text panel source-path completion dropdown powered by a local `CompletionEdit`.
- Completed: bounded global timeline buffer in UI state (`max_timeline_points`) to avoid unbounded growth during long runs.
- Completed: async integration tests for worker ready/bind/live updates, rebind history replay, ingest pause/resume, and discovery session snapshots.
- Completed: manual soak harness test for continuous stream ingest (ignored by default).
- Completed: UI-level soak runbook/checklist for long-running responsiveness and memory trend validation.
- Completed: discovery metadata polish (scope labels plus session host visibility).
- Completed: panel-type extensibility foundation (workspace panel kind list + centralized open/title/close policy routing).
- Completed: visual cleanup pass (list-style discovery rows without namespace duplication, reduced text-panel chrome/debug labels, human-readable timestamps in text/timeline panels).
- Completed: parameter panel entry UX refresh (node/path completion line edits with dropdown suggestions, compact load/apply flow).
- Completed: default namespace input completion sourced from discovered sessions.
- Completed: text payload area supports selection plus explicit copy action.
- Completed: centralized diff-based text-panel binding reconciliation replacing scattered manual bind calls.
- Completed: completion popups can open immediately on editor focus (`CompletionEdit::open_on_focus`), enabled for namespace/source/parameter entry fields in viewer.
- Completed: parameter panel auto-loads values on committed node/path entry (no extra `Load` step), with `Apply` retained for explicit writes.
- Completed: panel UI code split into individual modules under `app/panels/` and routed through a shared `Panel` trait-based interface.
- Completed: timeline revamp phase 1 with viewport zoom/pan, manual-interaction follow-live handoff, viewport decimation, and dynamic lane-based rendering (`X = time`, `Y = stream lanes`).
- Completed: timeline manual-window freeze semantics (absolute-time window while follow-live is off) with trim-time clamping to nearest retained range.
- Completed: timeline canvas contract update (`TimelineCanvasInput`/`TimelineCanvasOutput`) with timestamp-based scrub selection plus pan/zoom gesture outputs.
- Completed: timeline unit coverage for viewport math, manual-navigation handoff semantics, stream marker lifecycle, decimation properties, and pan/zoom mapping.
- Completed: timeline lane lifecycle hardening with inactive-first eviction policy tests and lane-scroll clamping tests.
- Completed: worker-side live event batching/coalescing to smooth high-frequency stream updates.
- Completed: compact source stats in text panels (durable bounds and ingest/durable frontiers).
- Completed: grouped `ViewerApp` state decomposition (`ShellState`, `DiscoveryState`, `TimelineState`, `WorkspaceState`, `RuntimeState`, `UiState`) and workspace-only `WorkspacePanel` model.
- Completed: persistence version bump for shell/workspace split with intentional reset of legacy all-tab dock state.
- Completed: bounded worker->UI event channel plus chunked history replay events (`StreamHistoryBegin/StreamRecordsChunk/StreamHistoryEnd`).
- Completed: event envelope sizing and per-frame worker-event ingest budgets in the app update loop.
- Completed: bounded UI->worker command channel with app-side pending command queue flushing.
- Completed: event-driven repaint scheduling for active backlog/follow-live activity (`~10 ms`) with idle polling removed, plus worker wake notifications for idle-to-active transitions.
- Completed: discovery steady-state patch events (`DiscoveryPatch`) with full snapshots kept for reconcile fallback.
- Completed: compact runtime diagnostics strip (frame timing, events/frame bytes, queued event depth, pending command depth).
- Completed: internal crate root split with thin `main.rs` and reusable `src/lib.rs` entrypoints (`ViewerApp`, `ViewerStartupOverrides`, logging setup).
- Completed: protocol/model split into `config.rs`, `discovery_types.rs`, and `protocol/{binding,commands,events}.rs`.
- Completed: panel developer API split (`PanelContext` + `UiIntent`) with centralized intent application in app command dispatch.
- Completed: module hierarchy split between shell panes (`app/panes/*`) and workspace panels (`app/workspace_panels/*`).
- Completed: app loop decomposition modules (`bootstrap`, `event_ingest`, `command_dispatch`, `shell`, `workspace`, `update_loop`).
- Completed: persistence key bump for internal architecture refactor (`workspace_dock_state_v7`, `ui_state_v6`) with intentional local state reset.

## 14. Known Limitations (Current Iteration)

1. No non-text stream visualization panel yet (for example 2D canvas).
2. Timeline viewport/lane scroll state is session-only and not persisted across app restarts.
3. Timeline lane universe is session-scoped (active bindings plus lanes with in-session samples).

## 15. Remaining TODOs

1. Add first non-text panel type (2D canvas MVP) using the existing workspace panel kind list.
2. Add panel-type specific discovery actions (e.g., open as text vs open as canvas) once second stream panel exists.

## 16. Future Alignment

1. Viewer currently reuses `SourceBindingRequest` / `SourceBindingInfo` directly and does not introduce an extra binding identity layer.
2. A dedicated cross-crate key-space/type alignment pass (`hulkz`, `hulkz-stream`, `hulkz-viewer`) is deferred to a future refactor.
