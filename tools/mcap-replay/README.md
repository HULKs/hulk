# MCAP replay for Twix

Serve recordings produced by `mcap_recorder` as ROS-Z topics over Zenoh.

In separate terminals, start playback and Twix:

```sh
cargo run -p mcap-replay -- recovered.mcap
./twix /replay --router tcp/127.0.0.1:7447
```

The binary runs its own Zenoh router by default, listening on
`tcp/0.0.0.0:7447`. Startup prints the reachable **Router URI** addresses and a
ready-to-copy Twix command. No separate router process is needed.

Choose another listen address or let the OS allocate a port:

```sh
cargo run -p mcap-replay -- recovered.mcap --listen tcp/127.0.0.1:0
```

To use an existing router instead, pass `--router`:

```sh
cargo run -p mcap-replay -- recovered.mcap --router tcp/127.0.0.1:7447
```

`--router` and `--listen` are mutually exclusive. The target namespace defaults
to `/replay` and can be changed with `--namespace`. Run one replay server per
namespace.

## Timeline panel

In Twix, click **+** in a tab bar and select **Timeline**. It is a normal dockable
panel: move it into a bottom split, duplicate it, or include it in a saved layout.

The server starts paused. A Blender-style timeline fills the panel: adaptive time
ruler, recording-range shading, a recording strip, and a full-height blue playhead
with its current timestamp. The compact header holds the transport controls:

- **Play/Pause** for real-time playback.
- Click or drag anywhere in the timeline to scrub immediately.
- Mouse wheel or pinch zooms around the pointer; middle-drag or Shift + wheel pans.
- **Frame all** (or double-middle-click) fits the complete recording.
- Start/end and backward/forward step buttons, with hover labels.
- A **⋯** menu for exact-time jumps, a saved step size, and copying the timestamp.
- Connection feedback and **Retry**.

With the timeline focused, **Space** toggles playback, **Left/Right** step,
**Shift + Left/Right** take a tenth-sized step, **Home/End** jump to the recording
bounds, and **F** frames the whole recording. Ruler labels are elapsed time, not
frame numbers: topics can have different sample rates. Zooming and panning do
not seek. Each panel has its own viewport.

Seeking pauses playback; playback also pauses at the end. All Timeline panels
share the same playback session. Other panels stay synchronized with seeks even
when the Timeline is hidden or closed.

## Topic and seek semantics

- Recorded topics are rooted under `--namespace` (default `/replay`). Both
  relative and absolute recorded names are placed beneath that namespace.
- Schemas are advertised through normal ROS-Z discovery and schema services.
  CDR payloads and source timestamps are preserved; transport publication IDs are
  newly assigned. Typed Twix panels require matching recorded type/schema hashes.
- A seek selects the last message at or before the requested **MCAP log time**
  independently for each topic. Equal log times are ordered by position in the
  file. A topic with no earlier message has no value.
- Each seek creates new publishers with retained snapshots. Twix resets its
  histories and presentation anchors and filters out previous-generation samples.
  Newly opened panels receive the retained snapshot even while paused.
- Histories grow from the selected position during playback. Independent topic
  snapshots do not guarantee matching acquisition timestamps for overlay panels.
- Only recorded topics are published. No image conversion, missing geometry, or
  parameter services are synthesized. In particular, `inputs/stereo_image_pair`
  does not become `inputs/left_image`.
- Recorded announcement payloads are unchanged, including their original
  publication IDs. This tool supports topic inspection, not resuming an
  announcement-correlated processing pipeline.

The reader requires chunk indexes and a summary, `ros-z-cdr` messages, and
`ros-z-schema-json` schemas. It reads/decompresses indexed chunks as needed and
caches up to 32 MiB of decoded messages and their vector storage; individual
chunks are limited to 64 MiB. All topics share the decoded chunk cache. Subsequent
seeks binary-search the cached messages instead of decompressing once per topic.
The forward playback reader is initialized on Play, not during scrubbing.
It does not load the complete recording into RAM. Recover/reindex files without
a summary before replaying them. Playback is not a full MCAP CRC integrity check.

## Control protocol

Twix uses JSON Zenoh queries at `hulk/replay/<namespace>/control` (for example,
`hulk/replay/replay/control` for `/replay`). An empty query reads status; a JSON
payload performs a command and returns the committed status:

```json
{"command":"seek","instance":"<instance from status>","position":1790276992055590764}
```

Commands are `play`, `pause`, and `seek`; `seek.position` is an absolute log-time
timestamp in nanoseconds. Every command includes `instance` to reject stale
commands after a server restart. Invalid commands receive a Zenoh error reply.
Status includes the file name, instance, generation, start/end/position,
playing state, and current publisher IDs indexed by absolute topic name.

Twix polls status at 10 Hz while connected. Commands bypass that interval and are
sent in the same frame as input. A pending read-only poll can be superseded by a
command; mutating requests are never cancelled. One command stays in flight plus
the latest pending seek, coalescing slider motion instead of queuing every pixel.
The thumb follows input immediately, even while an earlier seek is completing.
All networking happens off the UI thread.

## Checks

```sh
cargo test -p mcap-replay
cargo test -p ros-z-debug
cargo check -p twix --bin twix
```

To measure chunk lookup, publisher declaration, and seek-to-decoded-sample latency
using a real recording with a `camera_matrix` topic (separate router/client sessions):

```sh
MCAP_REPLAY_FILE="$PWD/recovered.mcap" cargo test -p mcap-replay scrub_latency -- --ignored --nocapture
```

This benchmark excludes UI rendering and reports median/max latency. Synthetic
tests also verify snapshot equivalence with MCAP's indexed reader, timestamp ties,
backward seeks, missing message indexes, and reuse of decoded chunks.
