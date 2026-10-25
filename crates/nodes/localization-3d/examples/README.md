# Localization replay

Replay current association and localization using recorded solve times and epoch
resets:

```sh
cargo run -p localization-3d --example relocalization_replay -- recording.mcap > replay.json
```

Change implementation or base parameters, rebuild, and replay the same file to
compare `audit_estimates`, `audit_frames`, lifecycle transitions and timings.
JSON goes to stdout; warnings go to stderr. Base parameters are embedded at build
time. There are no experiment modes or recording-specific checkpoints.

Required topics, each containing messages:

- `field_dimensions`
- `inputs/low_state`
- `robot_kinematics`
- `camera_geometry`
- `visual_odometry/current_left_camera_to_visual_odometer`
- `detected_objects`
- `localization/status`
- `debug/solve_diagnostics`

Missing topics, malformed input and missing required camera/attitude brackets
fail the replay. Field dimensions come exclusively from MCAP; changes within a
recording are rejected. Unrelated topics are ignored. Old-epoch and expired
observations are discarded, matching replay admission rules. Field association
waits for motion initialization; missing field estimates remain legitimate
localization results, not input errors.

This reruns association/localization, not image detection or visual odometry.
Recorded status supplies epoch resets, not Tracking/LostTrack decisions; those
are recomputed. Events preserve delivery order and source timestamps. Solves run
at recorded diagnostic delivery times regardless of the new solver's runtime.
Timing includes ingestion overhead and host scheduling, not just solver work.
Recorded trajectories are not ground truth.
