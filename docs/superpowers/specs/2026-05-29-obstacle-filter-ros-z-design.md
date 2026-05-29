# Obstacle Filter ros-z Port Design

## Context

The legacy `world_state::obstacle_filter` uses framework `PerceptionInput`
streams for Hydra object and pose detections, and `HistoricInput` streams for
camera matrices, odometry, and network robot obstacles. The ros-z port should
preserve those timing semantics without introducing a combined detection topic:
some downstream nodes consume only objects or only poses.

Network robot obstacles are replaced by player states produced from received
team messages. Robot clocks are not yet accurately aligned, so remote player
poses are timestamped with the local receive time of their `StateMessage`.

## Cache API Extension

Extend `ros-z` cache construction with:

```rust
with_stamped_entries(
    |message| -> impl IntoIterator<Item = (impl Into<Time>, CachedValue)>
)
```

The subscribed message type remains unchanged, while the resulting cache stores
`CachedValue`. Each received source message may insert zero, one, or many cache
entries. Each entry supplies its own timestamp. Empty iterators are valid and
insert nothing.

This supports `Players<Option<TimeWrapper<PlayerState>>>` by flattening present
players into timestamped `PlayerState` cache entries and filtering out `None`.

## Stream Timing

Use `ros-z-streams::FutureMap` only for the two Hydra detection outputs:

- `detected_objects`
- `detected_poses`

Both topics remain separate and become announcing publishers in `detection`.
Their announced timestamp is the source image timestamp used for inference.

`obstacle_filter` receives persistent fused entries from this two-stream
`FutureMap`. In normal operation both objects and poses are present for the same
timestamp because they come from the same inference. If one side is absent after
the safety boundary, the filter processes the present side and treats the
missing side as empty.

Player states are not part of the `FutureMap`. They are historic data, matching
the old `network_robot_obstacles` role.

## Player State Receiver

Add a ros-z `player_state_receiver` node. It subscribes to filtered network
messages and game controller state, maintaining the latest player states. It
publishes:

```rust
Players<Option<TimeWrapper<PlayerState>>>
```

Each updated remote player state is wrapped with the local receive time from
the `TimeWrapper<IncomingMessage>` that carried the `StateMessage`. Penalized
players are cleared as in the legacy node.

The whole `Players` structure remains available for later robotics processing.
Consumers that need timestamp-indexed player states can build a flattened cache
with `with_stamped_entries`.

## Obstacle Filter Data Flow

`obstacle_filter` is driven by persistent detection entries from the
`FutureMap`. For each detection timestamp it:

1. Predicts hypotheses using the nearest `current_odometry_to_last_odometry`
   cache entry, defaulting to identity if missing.
2. Queries the flattened `PlayerState` cache near the detection timestamp.
3. Converts remote player poses from field-relative to ground-relative using
   the available `ground_to_field` transform and updates hypotheses as
   `ObstacleKind::Robot`.
4. If `use_detected_objects` is enabled and a camera matrix exists near the
   timestamp, projects object and pose measurements to ground and updates
   hypotheses.

After processing persistent detection entries, it:

- removes timed-out and merged hypotheses using `node.clock().now()`;
- clears hypotheses when transitioning from penalized to unpenalized;
- removes unknown obstacles while not upright;
- publishes confirmed obstacles plus synthetic goal-post obstacles;
- publishes `obstacle_filter_hypotheses`.

`CycleTime` is not ported. Current-time behavior uses `node.clock().now()`.

## Removed Flow

Remove the ros-z `obstacle_receiver` flow from this stack. Its
`network_robot_obstacles` output is superseded by `player_state_receiver` and
the flattened player-state cache used by `obstacle_filter`.

This includes stopping `hulk_ros_z` from spawning `obstacle_receiver` once
`player_state_receiver` is wired in.

## Testing

Add focused tests for the shared timing primitives and ported behavior:

- `with_stamped_entries` inserts multiple independently timestamped cache
  entries from one source message.
- `with_stamped_entries` accepts empty iterators and respects cache capacity.
- Player-state flattening filters out `None` and stores only `PlayerState`
  values at their wrapped timestamps.
- Obstacle-filter pure helpers keep legacy behavior for measurement extraction,
  goal-post generation, hypothesis update, and hypothesis removal.

Verification should include `cargo fmt`, targeted Rust tests for touched crates,
and `pepsi --remote test` when feasible.
