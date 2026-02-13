# UI Design Guidelines for Responsive egui Apps

This document captures practical design guidelines for building fast, responsive, data-heavy egui applications.
It is written as a general-purpose guide and is not tied to a single project.

## Goals

Use these guidelines when you want an app that:
- Stays responsive during loading, decoding, network fetches, and indexing.
- Supports both native and web targets.
- Handles large datasets without freezing the UI thread.
- Remains debuggable and predictable as complexity grows.

## Core Principles

1. Keep a single source of truth for application data.
2. Make the UI thread an orchestrator, not a worker.
3. Move I/O and heavy compute off the UI thread.
4. Process work incrementally with strict per-frame budgets.
5. Use message passing and command queues for state mutations.
6. Apply backpressure so producers cannot overload consumers.
7. Repaint only when needed, but wake quickly when new data arrives.
8. Maintain derived indexes and caches incrementally.
9. Design explicitly for memory budgets and data eviction.
10. Make behavior observable with metrics and tracing.

## Recommended Architecture

Split your app into these layers:

- Authoritative state:
  - Domain data store(s), timeline data, metadata, session state.
  - This is what the UI reads from every frame.
- Derived state:
  - Caches, indexes, query accelerators, histogram summaries.
  - Rebuild incrementally from events, not by full scans each frame.
- Ephemeral UI state:
  - Panel toggles, hover/selection, filter text, open popups.
  - Keep this separate from domain state.
- Background workers:
  - File decode, parsing, network streaming, chunk fetch, export/save tasks.
  - Workers emit events; they do not directly mutate UI state.
- Message and command channels:
  - Data events from workers into app.
  - UI/system commands from widgets into app.

## Update Loop Pattern

Use a deterministic per-frame pipeline:

```rust
fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
    // 1) Advance time controls and animation clocks.
    self.update_time_state();

    // 2) Drain UI/system command queues (non-blocking).
    self.run_pending_commands();

    // 3) Ingest data events with a hard time budget.
    self.receive_data_with_budget(std::time::Duration::from_millis(8));

    // 4) Apply incremental index/cache updates.
    self.apply_store_events();

    // 5) Run memory maintenance and background prefetch scheduling.
    self.maintain_memory_budget();
    self.schedule_prefetch();
    self.receive_finished_prefetches();

    // 6) Render UI from current state.
    self.render_ui(ctx, frame);
}
```

Key idea: never let ingestion or maintenance consume the full frame. Stop and continue next frame.

## State Synchronization Model

Prefer unidirectional flow:

- Producers (file/network/background tasks) send immutable events.
- The app loop consumes events and mutates authoritative state.
- UI reads current state and emits commands.
- Commands are applied centrally in the app loop.

Benefits:
- Predictable ordering.
- Easier debugging and replay.
- Lower risk of race conditions from random cross-thread mutations.

## Keep UI Responsive Under Load

### Hard per-frame budgets

- Ingestion budget (for example 5 to 10 ms/frame).
- Background completion budget (for example max N completed tasks per frame).
- Cache maintenance budget.

If a budget is exceeded:
- Stop work.
- Request repaint.
- Resume next frame.

### Never block on UI thread

Do not do these in `update`:
- Blocking file reads.
- Blocking network calls.
- Large decode loops without yielding.
- Large memory frees that can be moved off-thread.

### Progressive rendering

- Show partial data as soon as available.
- Keep controls usable while data continues loading.
- Use loading indicators tied to real source/task states.

## Async and Worker Design

Use a runtime abstraction so native and web use the same high-level flow:

- Native: spawn on tokio/rayon/threads depending on task type.
- Web: use non-blocking browser futures; do not rely on blocking calls.

Task categories:
- I/O-bound: dedicated thread/task, chunked reads, streaming decode.
- CPU-bound: worker pool (for example rayon), avoid monopolizing one worker.
- Mixed: pipeline as stages and hand off between pools.

## Channels and Backpressure

Use bounded channels based on byte size, not only message count.

Why:
- One message can be tiny or very large.
- Byte-bound channels prevent OOM under bursty producers.

Guidelines:
- Tag messages with approximate size.
- Apply sender backpressure when over budget.
- Track current in-flight bytes and queue depth.
- Use explicit end-of-stream/quit markers.
- Add optional flush semantics for "drain before continue" operations.

## Data Loading and Streaming

General loading strategy:

1. Validate obvious errors synchronously (missing file, malformed URI).
2. Start loaders asynchronously.
3. Stream decoded records/chunks incrementally into event channel.
4. Keep app interactive while data arrives.

For large remote datasets:
- Load lightweight manifest or index first.
- Present data immediately where possible.
- Fetch heavy chunks on-demand based on current view/time.
- Prioritize chunks near the active cursor/viewport.
- Cancel outdated in-flight requests when user context changes.

## Caching and Indexing Strategy

Build caches around observed hot paths:
- Timeline histograms.
- Entity/component lookup indices.
- View-specific query caches.
- Decoded media/frame caches.

Rules:
- Update caches from store events incrementally.
- Avoid full cache rebuilds in frame loop.
- Separate cache lifetime from UI widget lifetime.
- Record cache hit/miss and memory usage.

## Memory Management

Treat memory as a first-class runtime constraint:

- Define a configurable memory limit.
- Track both resident memory and tracked internal allocations.
- When over budget, purge in controlled fractions.
- Keep a protected working set around current interaction context.
- If data is re-fetchable, allow more aggressive eviction.
- Move expensive deallocation off the UI thread where possible.

## Repaint Strategy

Request repaint for:
- New incoming data.
- Active animations/playback.
- Task completion that changes visible state.

Avoid permanent repaint loops when idle.

Useful pattern:
- Data receiver sets a wake callback to trigger `request_repaint_after(small_delay)`.
- Time controls request repaint only while playing.
- Following/live modes repaint on new data, not continuously.

## UX Considerations for Data Apps

- Loading screen should show source identity and status.
- Allow opening, switching, and closing sources while others load.
- Keep selection and navigation stable when data updates.
- Do not block all UI because one source is busy.
- Make async actions visible with progress or state chips.

## Command System Design

Use explicit command types for system mutations:

- `LoadDataSource`
- `AddReceiver`
- `SetSelection`
- `TimeControlCommands`
- `Save/Export` tasks

Process commands in a central place each frame.
This gives:
- Serialization of side effects.
- Better tracing and testability.
- Cleaner widget code (widgets emit intent, app performs action).

## Error Handling

Split errors into:
- Immediate user errors (invalid input, missing path).
- Deferred task errors (network timeout, decode failure).

Guidelines:
- Fail fast for synchronous validation errors.
- Log rich context for async failures.
- Surface user-facing errors through non-blocking notifications.
- Keep stream alive when possible after non-fatal errors.

## Observability and Diagnostics

Instrument these by default:
- Frame time and long-frame counts.
- Queue depth and in-flight bytes per source.
- Data ingest throughput.
- Background task counts and durations.
- Prefetch hit ratio and cancellation count.
- Cache memory and hit/miss metrics.

Add tracing spans around:
- Ingest loops.
- Decoding.
- Cache updates.
- Prefetch planning and completion.

## Native vs Web Considerations

Expect different constraints:

- Native:
  - Blocking and threaded work is available.
  - You can use bounded blocking channels.
- Web:
  - No blocking operations.
  - Fewer threading options (depending on target features).
  - Prefer unbounded/non-blocking send paths with explicit budgeting in the consumer loop.

Design APIs so the same app logic works on both targets behind a thin platform layer.

## Testing and Validation

Recommended tests:
- Snapshot tests for core UI states.
- Integration tests that pump command + data event sequences.
- Stress tests for bursty input streams.
- Memory-limit tests that verify purging and continued responsiveness.
- Deterministic playback/time-control tests.

Also test:
- Rapid source switching.
- Partial loads and cancellation.
- Repeated open/close cycles (resource leak checks).

## Practical Checklist

Before shipping a data-heavy egui app, confirm:

- UI thread never performs blocking I/O.
- Event ingestion is budgeted per frame.
- All heavy tasks are off-thread/off-main-loop.
- Message channels have backpressure or capacity controls.
- Repaint policy does not spin when idle.
- Caches are incremental and measurable.
- Memory pressure triggers controlled purging.
- Remote prefetch is prioritized and cancelable.
- Errors are surfaced without freezing interaction.
- Metrics exist for frame time, queue depth, and throughput.

## Closing Notes

Immediate-mode UI can scale to complex, high-throughput apps if you:
- keep state architecture strict,
- process work incrementally,
- enforce budgets and backpressure,
- and treat observability and memory as core design concerns.

These patterns are broadly applicable to egui apps for robotics, visualization, media, telemetry, and other streaming/data-centric domains.
