# hulkz-stream: Consistent User Stories and Requirements

## 1. Purpose

`hulkz-stream` is an async stream backend for robotics debug visualization and replay tooling.

It must support:

- low-latency live visualization,
- durable long-horizon recording,
- timeline-driven historical queries,
- loading previously recorded storage from other tools/binaries.

## 2. Product Principles

1. Disk is source of truth for history; memory is a transparent acceleration layer.
2. Memory is bounded; durable history is practically unbounded (for this iteration, assume disk is effectively infinite).
3. Storage is global (shared across all sources), not per subscription handle.
4. Ingest must support multiplexing by logical source identity.
5. Live visualization must not require disk round-trip for new samples.
6. APIs and source identity must align with hulkz concepts: planes, scopes, namespaces, nodes, paths.
7. Stats for UI should reflect full durable history bounds, not only RAM cache bounds.

## 3. Scope and Non-Scope

### In scope

- Async-only backend crate.
- Global durable storage (MCAP default; pluggable storage abstraction allowed).
- Concurrent read/write access (recording + querying at same time).
- Transparent RAM cache with byte budget.
- Per-source stream handles and source/time query APIs.
- Namespace-aware source binding (follow target namespace vs pinned namespace).
- Timeline information APIs for rich UI rendering.

### Out of scope (this iteration)

- Full-disk handling strategies (stop/drop/evict on disk full).
- Distributed or replicated storage.
- Advanced query language beyond source/time primitives.
- UI implementation itself.
- Runtime wrapping ergonomics for sync callers.

## 4. Component Model

`hulkz-stream` is a multi-component system in one crate boundary.

1. Live Ingest Component

- Maintains deduplicated network subscriptions by source identity.
- Emits samples to live fast path and durable writer path.

2. Durable Writer Component

- Appends records to on-disk global store.
- Updates durable indexes/frontiers for historical queryability.

3. Query Engine Component

- Resolves source/time queries against cache first, then durable store.
- Supports latest, before, nearest, and range.

4. Cache Component

- Byte-budget constrained, transparent, read/write-through acceleration.

5. Timeline/Stats Component

- Exposes per-source and aggregate timeline metadata for UI rendering.
- Exposes both durable history bounds and ingest frontier.

6. Session/Control Component

- Handles backend target namespace and connection gating.
- Preserves stream handle validity across control changes.

## 5. Core Definitions

- Source Identity: tuple aligned with hulkz routing semantics:
  - plane kind (`data`, `view`, `param-read-updates`),
  - scoped path,
  - node override (for private scope resolution),
  - namespace binding mode (`follow-target` or `pinned(namespace)`).
- Record: raw sample + metadata: source identity, timestamp, encoding, payload.
- Durable Frontier: latest timestamp that is known persisted for a source (or globally).
- Ingest Frontier: latest timestamp observed by live ingest (may be ahead of durable frontier).
- Timeline Bucket: aggregated interval metadata for UI drawing (count and timestamp min/max per bucket).

## 6. User Stories and Acceptance Criteria

### US-001 Low-Latency Live Visualization

As a debug UI developer, I want live updates to appear quickly while recording is active, so I can monitor robot behavior in near real-time.

Acceptance criteria:

1. Newly ingested samples become queryable through live path without waiting for disk re-read/reparse.
2. Querying latest data during recording does not require file system notifications or polling MCAP changes.
3. End-to-end live latency target is bounded by ingest + in-memory processing (not disk read speed).

### US-002 Durable Long-Horizon Rewind

As an analyst, I want to scrub far back in time after long sessions.

Acceptance criteria:

1. Ingested records are durably appended to on-disk storage.
2. Historical queries can retrieve data older than RAM cache horizon.
3. Rewind depth is not constrained by memory policy.

### US-003 Passive Watching with Bounded Memory

As an operator leaving visualization open for long periods, I want stable memory usage.

Acceptance criteria:

1. Cache has a strict configurable byte budget.
2. Cache evicts automatically when budget is exceeded.
3. Eviction never deletes durable history.

### US-004 Global Store for All Sources

As a system integrator, I want one storage backend for all sources.

Acceptance criteria:

1. Global durable store holds records from all subscribed sources.
2. Subscriptions/handles do not create isolated per-handle stores.
3. Query APIs can filter by source identity from the global corpus.

### US-005 Multiplexed Ingest by Source

As a backend operator, I want duplicate subscriptions to share one network subscription.

Acceptance criteria:

1. Equivalent source identities deduplicate to one active subscriber/driver.
2. Multiple handles on same source share ingest and source-level stats.
3. Driver shuts down when no handle for that source remains.

### US-006 Namespace-Aware Switching

As an operator, I want to switch target namespace at runtime with predictable behavior.

Acceptance criteria:

1. Follow-target sources rebind to new effective namespace on switch.
2. Pinned sources remain on pinned namespace.
3. Existing stream handles remain usable after switch.

### US-007 Durable History Stats for UI

As a timeline UI developer, I want stats that describe full recording extent.

Acceptance criteria:

1. `oldest/latest/len` represent durable history bounds (not cache-only bounds).
2. API also exposes ingest frontier to indicate live head that may be ahead of durable commit.
3. UI can detect and render ingest-vs-durable lag.

### US-008 Rich Timeline Rendering Support

As a UI developer, I want timeline-oriented metadata so I can draw dense, expressive timelines.

Acceptance criteria:

1. API provides bucketed timeline summaries over time ranges.
2. Bucket summaries include at least message count and time bounds.
3. Queries support per-source and aggregate timeline views.

### US-009 Load Existing Recordings

As a developer, I want to open storage recorded by a different process/binary.

Acceptance criteria:

1. Backend supports read-only open of existing storage.
2. Backend supports read-write live mode (record + query concurrently) on managed storage.
3. Query semantics are identical regardless of recording origin.

## 7. Functional Requirements

## FR-A Async API and Lifecycle

- FR-A01: Crate API is async-only for this iteration.
- FR-A02: Backend construction is async and runtime-native (no internal sync wrapper requirement).
- FR-A03: Clean shutdown flushes durable writes and closes resources.

## FR-B Storage Engine

- FR-B01: Durable storage format defaults to MCAP (or equivalent pluggable backend with MCAP implementation).
- FR-B02: Durable records include source identity, timestamp, encoding, payload.
- FR-B03: Storage is restart-safe and reopenable.
- FR-B04: Writes acknowledged as durable must survive process restart/crash.
- FR-B05: Storage layout uses rolling/segmented files with index support (recommended for concurrent read/write and recovery).

## FR-C Source Identity and hulkz Alignment

- FR-C01: Source identity must encode hulkz plane/scope/namespace/node/path semantics correctly.
- FR-C02: Dedup identity includes namespace binding mode (`follow-target` vs `pinned`).
- FR-C03: Follow-target and pinned sources must not deduplicate together.

## FR-D Ingest and Multiplexing

- FR-D01: One deduped source identity maps to one active ingest subscriber.
- FR-D02: Multiple handles for same source share source state/stats.
- FR-D03: Live ingest emits records to both fast in-memory path and durable writer path.
- FR-D04: Connection gating can pause/resume ingest without invalidating handles.
- FR-D05: Writer queue saturation uses deterministic blocking backpressure (no silent drops).

## FR-E Query Semantics

- FR-E01: Query operators required: latest, before-or-equal, nearest, inclusive range.
- FR-E02: Query APIs are source/time based.
- FR-E03: Cache miss fallback to durable store is transparent.
- FR-E04: Out-of-range historical queries return `None`/empty collections, not hard errors.
- FR-E05: Durable query execution must be index-driven (`source + timestamp`) so lookup cost scales with query selectivity rather than total recording size.

## FR-F Cache Layer

- FR-F01: Cache budget is global and byte-based.
- FR-F02: Cache policy is configurable from backend builder.
- FR-F03: Cache eviction is deterministic.
- FR-F04: Cache is a performance layer only; correctness cannot depend on cache residency.
- FR-F05: Prefetch hooks exist for timeline scrub acceleration.
- FR-F06: Cache eviction should be scrub-aware via a configurable scrub working-set window; data inside that window should be evicted last when possible.

## FR-G Namespace and Control Semantics

- FR-G01: Backend exposes runtime target namespace updates.
- FR-G02: Follow-target sources transition to new effective source on namespace change.
- FR-G03: Pinned sources ignore target namespace changes.
- FR-G04: Control transitions preserve handle validity.

## FR-H Stats, Frontiers, and Observability

- FR-H01: Expose durable history bounds per source (`oldest`, `latest`, `len`).
- FR-H02: Expose ingest frontier per source (live head).
- FR-H03: Expose durable frontier per source.
- FR-H04: Expose source `last_error` for ingest/storage/query failures.
- FR-H05: Expose source stats as snapshot and watch stream.
- FR-H06: Expose cache metrics: hit/miss/eviction counts and current bytes.
- FR-H07: Expose deduped active source/subscriber counts.
- FR-H08: Expose writer queue depth/high-watermark and backpressure event counters.

## FR-I Timeline APIs

- FR-I01: Provide bucketed timeline summary query API for a time range.
- FR-I02: Timeline API supports per-source and aggregate modes.
- FR-I03: Bucket response includes at least `bucket_start`, `bucket_end`, `message_count`, `min_ts`, `max_ts`.
- FR-I04: Timeline queries work while ingest is active.

## FR-J Concurrency and Access Modes

- FR-J01: Concurrent writer + reader operation on same storage must be supported.
- FR-J02: Read-only mode for existing recordings must be supported.
- FR-J03: Read-write mode for live recording + querying must be supported.
- FR-J04: Live query path must not rely on filesystem change notifications or file reparsing loops.

## FR-K Time and Versioning Semantics

- FR-K01: Public API timestamps remain `hulkz::Timestamp`.
- FR-K02: If timestamp identity metadata is unavailable, reconstruction falls back to system-clock-based timestamp creation with an explicit fallback clock id.
- FR-K03: Manifest schema versions must be validated against a single crate constant (no inline magic versions).

## 8. Non-Functional Requirements

- NFR-001: Hot-path latest queries should be suitable for interactive dashboards.
- NFR-002: Historical disk queries should be usable for step/scrub workflows.
- NFR-003: Memory usage remains bounded by configured cache budget.
- NFR-004: Behavior is deterministic across reconnects, namespace changes, and restarts.
- NFR-005: Ingest path avoids duplicate network subscriptions for equal source identities.

## 9. Durability and Frontier Semantics

1. Live-first visibility is allowed:

- A record may be visible via live path before durable commit completes.

2. Durable history stats are durable-only:

- `oldest/latest/len` represent committed durable data.

3. Ingest-vs-durable gap is explicit:

- API exposes ingest frontier and durable frontier so callers can reason about lag.

## 10. Test and Validation Matrix

### FR Traceability (Current)

| Requirement IDs | Coverage Tests |
|---|---|
| FR-C02, FR-C03, FR-D01, FR-D02, FR-H07 | `backend_deduplicates_equivalent_sources` |
| FR-G01, FR-G02, FR-G04 | `follow_target_namespace_switch_keeps_handle_valid` |
| FR-D04 | `ingest_gating_pauses_and_resumes_source_updates` |
| FR-D05, FR-H08 | `writer_backpressure_metrics_are_observable` |
| FR-I01, FR-I03, FR-I04 | `source_timeline_is_available_during_active_ingest` |
| FR-F05 | `prefetch_range_cancellable_respects_cancel_token` |
| FR-J01, FR-J03, FR-J04 | `concurrent_query_during_ingest_is_stable` |
| FR-H02, FR-H03 | `ingest_frontier_can_lead_durable_frontier` |
| FR-B03, FR-B04 | `reopen_after_ungraceful_drop_recovers_latest_data` |
| FR-E01, FR-E04, FR-J03 | `storage_query_operators_roundtrip` |
| FR-B03, FR-B05, FR-E01, FR-E05 | `indexed_queries_remain_correct_after_segment_roll_and_restart` |
| FR-E05, NFR-002 | `indexed_query_smoke_with_many_segments` |
| FR-J02, FR-E03, FR-C01 | `load_external_mcap_best_effort` |
| FR-C03 | `source_key_distinguishes_namespace_binding` |
| FR-F03 | `evicts_oldest_timestamp_first` |
| FR-F06 | `scrub_window_protects_recent_scrubbed_data` |
| FR-E01 | `nearest_prefers_earlier_on_tie` |
| FR-K02 | `timestamp_fallback_is_explicit_and_stable`, `timestamp_id_metadata_roundtrip` |

## 11. Future Extensions (Post-Iteration)

- Runtime wrappers for sync callers.
- Explicit disk quota/full-disk policies.
- Advanced compaction and retention policies for durable storage.
- Additional timeline analytics (rate curves, sparse intervals, anomaly markers).

## 12. Implementation Style and Rust Idiomatic Preferences

This section captures implementation style expectations for consistency, maintainability, and idiomatic Rust.

### 12.1 Async-First and Runtime-Native
- IS-001: Keep the crate async-first in this iteration; avoid introducing hidden synchronous wrappers.
- IS-002: Avoid blocking operations in async contexts (`std::fs` on hot async paths, long mutex holds, etc.).
- IS-003: Background processing should be explicit (actors/drivers/tasks with clear ownership).

### 12.2 Ownership and Drop-Driven Lifecycle
- IS-004: Prefer ownership and RAII over manual lifecycle bookkeeping where practical.
- IS-005: Use drop semantics for teardown of ephemeral resources (e.g., leases/handles ending source drivers).
- IS-006: Keep explicit graceful shutdown APIs for durable flushing/closing where RAII alone is insufficient.

### 12.3 Builder Patterns and Configuration Clarity
- IS-007: Public configuration should use typed builders with documented defaults.
- IS-008: Builders should express immutable setup; runtime mutability should only occur via explicit control APIs.
- IS-009: Where options interact (e.g., dedup and source identity), precedence/merge semantics must be deterministic and documented.

### 12.4 Concurrency and Coordination
- IS-010: Prefer message-passing and actor boundaries for cross-task coordination.
- IS-011: Keep shared mutable state minimal and behind clear synchronization primitives.
- IS-012: Keep lock scope short and avoid lock contention between ingest, query, and writer paths.

### 12.5 Error Handling and Health Signaling
- IS-013: Use typed errors with context; avoid opaque string-only propagation at API boundaries.
- IS-014: Surface operational faults via health/stats channels (`last_error`, frontiers, metrics), not silent drops.
- IS-015: Reserve panic/assert for internal invariant violations, not recoverable runtime conditions.

### 12.6 API Predictability and Semantics
- IS-016: Preserve semantic consistency independent of cache hit/miss state.
- IS-017: Keep handle types cheaply clonable and ergonomic for UI/application layers.
- IS-018: Clearly distinguish ingest frontier vs durable frontier semantics in naming and docs.

### 12.7 Performance-Oriented Rust Practices
- IS-019: Minimize copies in hot paths; keep payload handling allocation-aware.
- IS-020: Prefer zero-copy/borrowed reads where safe and compatible with async ownership.
- IS-021: Separate fast live-read path from slow durable path to avoid disk-induced head-of-line blocking.

### 12.8 Testing Discipline and Evolvability
- IS-022: Add tests alongside behavior changes (unit + integration + restart/concurrency scenarios).
- IS-023: Breaking changes are acceptable in prototype phase but must be documented in this requirements file.
- IS-024: Keep component boundaries reusable so standalone recorder mode can be added without architectural rewrites.
