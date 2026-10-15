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

## Options

Pass options after `--` in the Cargo command above.

| Option | Purpose |
| --- | --- |
| `--namespace /replay` | Namespace to use in Twix; defaults to `/replay`. |
| `--listen tcp/127.0.0.1:0` | Choose a router listen address; port `0` selects an available port. |
| `--router tcp/127.0.0.1:7447` | Connect to an existing router instead of starting one. |
| `--index-cache-mib 512` | Increase the index memory budget if queries exceed the default 256 MiB. |

`--listen` and `--router` are mutually exclusive. Use `--help` for all options.

## Playback

In Twix, open **+ → Timeline**, then add the panels you want to inspect.

- Click or drag to seek; seeking pauses playback.
- Use the header buttons to play, pause, step, or jump to the start/end.
- Wheel/pinch zooms; middle-drag or Shift + wheel pans. **Frame all** fits the recording.
- **⋯** offers exact-time jumps, step size, and timestamp copying.

Playback is independent for each Twix client.
See the [Twix timeline guide](../../docs/tooling/twix.md#mcap-timeline) for keyboard shortcuts.
