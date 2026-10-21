# MCAP replay for Twix

Open an indexed ROS-Z recording:

```sh
cargo run -p mcap-replay -- recording.mcap
```

The server starts a Zenoh router and prints a Twix connection command:

```sh
./twix /replay --router tcp/127.0.0.1:7447
```

Recordings need a summary and chunk indexes. Recover/reindex incomplete recordings first.

## Watch a directory

```sh
cargo run -p mcap-replay -- /data/recordings --index-cache-mib 1024
```

Existing and newly added `.mcap` files are served recursively using `notify`.
Changes reload recordings; deletions withdraw them.
Symlinks are not followed.
Namespaces follow relative paths: `game1/robot2.mcap` becomes `/replay/game1/robot2`.
`--namespace` changes the prefix, which must be `/replay` or start with `/replay/`.
Letters, digits, underscores, hyphens, and dots are preserved; other filename bytes (including `~`) are escaped as `~xx` to produce valid, collision-free namespaces.
Each recording prints its Twix command.

Unreadable or incomplete files are logged and retried on subsequent filesystem changes without stopping other recordings.
Duplicate open errors are suppressed.
Write recordings to a temporary name and rename them to `.mcap` when complete to avoid exposing partially written files.

The index budget is shared across recordings, including active and building timestamp indexes.
Idle indexes are evicted globally in least-recently-used order; queries that cannot fit return errors.
Index builds are serialized across recordings, with parallel scanning within each build.
Missing indexes wait asynchronously for the build mutex; a cancellation token stops that wait without polling.
Running builds retain their owned mutex guard until they finish and check the same cancellation token while scanning.
This is not a total memory limit: summaries, schemas, and decoded chunks use additional memory.
Dropping a recording queues index removal to one Tokio blocking task, without taking the cache lock.
Scanning and sorting run outside that lock, so cleanup and cached lookups can proceed during builds.
Removed allocations remain charged until freed.
Shutdown signals all recordings before waiting for them, then drains the cleanup queue and awaits the cleanup task.

## Options

Pass options after `--` in the Cargo command above.

| Option | Purpose |
| --- | --- |
| `--namespace /replay` | Single-file namespace or directory prefix; must be `/replay` or below it. |
| `--listen tcp/127.0.0.1:0` | Choose a router listen address; port `0` selects an available port. |
| `--router tcp/127.0.0.1:7447` | Connect to an existing router instead of starting one. |
| `--index-cache-mib 1024` | Global timestamp-index memory budget; defaults to 1 GiB (1024 MiB). |

`--listen` and `--router` are mutually exclusive. Use `--help` for all options.

## Playback

In Twix, open **+ → Timeline**, then add the panels you want to inspect.

- Click or drag to seek; seeking pauses playback.
- Use the header buttons to play, pause, step, or jump to the start/end.
- Wheel/pinch zooms; middle-drag or Shift + wheel pans. **Frame all** fits the recording.
- **⋯** offers exact-time jumps, step size, and timestamp copying.

Playback is independent for each Twix client.
See the [Twix timeline guide](../../docs/tooling/twix.md#mcap-timeline) for keyboard shortcuts.
