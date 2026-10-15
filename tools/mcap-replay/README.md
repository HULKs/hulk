# MCAP queries for Twix

Serve indexed ROS-Z recordings to query clients:

```sh
cargo run -p mcap-replay -- recovered.mcap
```

The server starts an embedded Zenoh router, prints its reachable addresses, and
prints a Twix command. `--listen tcp/127.0.0.1:0` chooses an available port.
Alternatively, `--router tcp/127.0.0.1:7447` connects to an existing router.
These options are mutually exclusive. `--namespace` defaults to `/replay`.

The `RemoteMcap` API can query this service; Twix recording playback is added separately.

Results include a half-open cursor validity range. While the cursor stays inside
it, the remote backend reuses the last result, including for backward seeks;
images are transferred when selection can change, not on every playback tick.
Validity includes joined records and missing-match diagnostics. Live reports the
same range for its current inputs but continues reevaluating on input updates.
A shared catalog check detects replaced recordings even while playback is paused.

## Queries and timestamps

Clients discover the recording through `ros_z_debug::query::RemoteMcap`. No recorded
topics are republished. The backend lists recorded topics and schemas and returns
raw CDR records or computed timestamps in response to one query plan:

- `AtOrBefore(n)`: last n eligible records at or before the anchor; `latest()` means n = 1.
- `After(n)`: first n eligible records strictly after the anchor.
- `Exact`: last record at the exact timestamp.
- `Bracket { max_gap }`: one exact record or the surrounding pair within the whole-span limit.
- `Window { before, after }`: all records in an inclusive duration window around the anchor.

Timestamp extraction, `available(...).min()`, and empty-result fallback compose selections into a single plan. Optional attachments report their outcomes independently. `Source::select(...).requiring(inputs)` filters candidates before selection; `.attach(inputs)` independently resolves inputs afterward. All subscriptions consume plan results directly.

Each observation chooses source time (MCAP `publish_time`) or a schema-checked
payload timestamp projection. Projected values are checked integer ROS-Z times;
publication and log timestamps are retained separately. Projection does not
synchronize clocks.

The Image panel uses developer-owned projections: `.header.stamp` for images and
`TimeWrapper.time` for stamped overlay inputs. Image/overlay projection menus and
legacy saved overrides are ignored. The Text panel always uses source time.
Field paths support fields, indices, and
enum payloads, such as `.time`, `.header.stamp`, `.samples[0].time`, or `.state::Ready.time`.

When `inputs/left_image` is absent, the Image panel observes `inputs/stereo_image_pair` and maps its decoded value to the left image.
The stereo query projects `.left.header.stamp`, so its overlays still join at the left image's acquisition time.
If the primary topic exists, it wins even when it has no eligible samples.
Live publisher discovery promotes/demotes the selected source automatically; replay uses the recording catalog.

The image panel submits one plan selecting the closest image at or before the timeline cursor, with optional exact detections and bounded-bracket geometry. The server searches the entire recording for matches, including records published or logged after the cursor. An image without matching detections stays visible without boxes. Partial plan responses can show that image before optional indexes finish; later responses refine it without moving the cursor.

Live uses the same operators with a different application-built primary expression: exact image at the minimum available detection timestamp, falling back to the latest image. Selection and matching use the same retained source views, eliminating per-frame asynchronous query chasing. Missing optional inputs never gate the image.

### Backend contract

`Backend` exposes `topics()` and `observe(cursor, plan)`; typed decoding sits above raw observations.
Plans use the same pure evaluator over immutable Live views and pinned recording indexes; timestamp expressions do not load payloads.
`CursorHandle` clones share a cursor; separately constructed handles are independent.
Live uses the session clock. Explicit seeks increment the revision; continuous playback advances within it so slow queries can deliver useful progress.
Results identify their evaluation cursor, and installation rejects obsolete revisions.
Typed caches retain successful and failed decodes and keep the last resolved data until a definitive replacement arrives; a resolved empty result clears it.
`query_status()` exposes freshness, dependency failures, missing matches, and retention limits independently of panel warning UI.
Equal-time selections choose the last record in stable source order, including both bracket endpoints; zero-width windows retain all duplicates.
Inputs are relative to the primary record, and its cursor does not limit their publication/log times.
Compatible Live subscriptions and projections share bounded histories/indexes while respecting acquisition policy; dropping the last handle releases background work.
Live ingests without UI frames, evaluates snapshots on demand, and invalidates them on feed/catalog changes; delayed arrivals can refine ready results.
Parameter services remain a separate Live capability. Persistent indexes and cross-observation atomic presentation are deferred.

## Indexing and limits

The reader requires chunk indexes, a summary, `ros-z-cdr` messages, and
`ros-z-schema-json` schemas. Recover/reindex recordings without a summary first.
Repeated recorded topic names are rejected rather than silently mixing schemas.

Opening reads summary metadata only; it does not scan messages or build timestamp indexes.
Initial timeline bounds are the summary's log-time bounds and expand as queried indexes provide effective-time bounds.
Source time and payload projections are both indexed lazily for requested channels.
Plans build needed indexes progressively, publishing usable results between builds so optional indexes cannot gate the primary. Unused fallback alternatives are not indexed. The performance benchmark exercises this progressive plan path, not an eager pre-indexing wrapper.
Each cold build uses up to four scoped workers to read, decompress, and project independent chunks.
Positional reads give workers independent cursors on the same open recording.
Workers merge bounded batches under one short-lived lock; index allocation, eviction, and the shared byte budget remain coordinated.
Failed builds stop their workers and discard incomplete indexes without cancelling subsequent queries.
Source-time indexing reads message headers without CDR decoding; payload projection uses selective timestamp extraction.
The catalog remains available while the recording worker builds indexes.

Each sorted Vec entry occupies 16 bytes: an integer nanosecond timestamp, a 32-bit chunk ID, and a 32-bit uncompressed record offset.
Entries contain no payloads or per-message schema metadata.
File location breaks equal-time ties deterministically.
Queries binary-search the entries directly without copying timestamp arrays, then load selected payloads grouped by chunk.
`ros_z_debug` exposes pending, indexing, ready, and failed states independently of panel rendering.

Payload chunks use a 32 MiB cache, with a 64 MiB per-chunk limit.
Timestamp indexes use a shared LRU cache with a default 256 MiB entry budget and a 256-index limit.
Use `--index-cache-mib <MiB>` to change the budget.
Allocated vector capacity is counted, including indexes under construction.
Indexes required by the current query are pinned; unused indexes are evicted as needed.
If an index cannot fit, that dependency fails explicitly while the server remains available; optional dependency failures do not suppress a usable primary.
Responses are
bounded to 4,096 selected record references per plan and a 64 MiB encoded response; selected distinct payloads
are limited to 32 MiB. Row-limited results report truncated coverage and retain
the newest interval rows. Oversized payload requests fail explicitly. Live
histories default to 4,096 samples and 128 MiB per observed topic, shared across
observations; their coverage distinguishes live data from retention-limited data.

## Protocol and checks

The Zenoh queryable is `hulk/mcap/<namespace>/query` (or `hulk/mcap/query` at root). Requests are JSON `Request`
values and replies are size-limited binary `Response` values defined in
`crates/ros-z-debug/src/query/remote.rs`. Only catalog and plan requests are supported. A plan request can receive multiple `PlanData` updates until dependencies settle. The catalog supplies a recording-instance identity; queries must include it to reject data from a replaced recording. Twix and replay are built and deployed together; the internal protocol has no compatibility guarantees.

```sh
cargo test -p ros-z-debug -p mcap-replay -p twix
cargo clippy -p ros-z-debug -p mcap-replay -p twix --all-targets -- -D warnings
```

To measure metadata-only startup, cold source/projected indexing, and repeated in-process seeks on the recovered recording:

```sh
MCAP_REPLAY_FILE="$PWD/logs/recovered.mcap" cargo test -p mcap-replay recorded_file_lazy_index_performance -- --ignored --nocapture
```

This check requires `camera_matrix` and `detected_objects` topics and excludes network and UI rendering time.
For a serial control run of the same implementation, set `MCAP_INDEX_WORKERS=1` when invoking the test; this override is test-only.

Verify stereo-to-left mapping, pixel decoding, and delayed overlays through Twix's actual Image panel:

```sh
cargo test -p twix sustained_live_plan_presents_matching_overlays_without_cursor_chasing
```

### Historical indexing measurements

Two experiments used `logs/recovered.mcap` (2.79 GiB compressed, 4.51 GiB uncompressed, 27 topics).
Each used fresh in-memory indexes, warm OS file caches, and six rotating/reversed rounds after warm-up, excluding compilation, network, and rendering.
The stages indexed source-time `camera_matrix`, then payload-time `detected_objects` joined with `camera_matrix`.
Medians below are from separate comparisons; source-time variation was substantial, so they are not universal regression percentages.

| Experiment | Variant | Source-time indexing | Projected join indexing |
| --- | --- | ---: | ---: |
| Buffering | Baseline | 3.856 s | 4.025 s |
| Buffering | 256 KiB `BufReader<File>` | 5.345 s | 4.164 s |
| Buffering | Buffer plus whole-chunk CRC prevalidation | 5.444 s | 4.100 s |
| Workers | Previous sequential scanner | 5.059 s | 4.132 s |
| Workers | New implementation, one worker | 5.587 s | 4.750 s |
| Workers | New implementation, four workers | 1.578 s | 1.487 s |

Buffering did not establish a speedup and was reverted; checksum verification was never disabled.
All 3,438 chunks had CRCs, so the combined variant exercised whole-chunk prevalidation.
Four workers improved the measured medians by approximately 3.2×/2.8× over the previous scanner.
Metadata startup remained about 8–10 ms, and the three requested indexes occupied 28,187,568 bytes (26.9 MiB).
These historical runs used the retired eager/batched query API; projected-index timings are not directly comparable to the current progressive plan benchmark above.
