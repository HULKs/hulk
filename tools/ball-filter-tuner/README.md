# Ball filter optimizer

## Availability: experimental worktree addition

This README is a docs-only addition. Main has no `ball-filter-tuner` crate,
simulator implementation or root `simulator` launcher. Simulator development
lives on [Alex's `motion-inference-simulator` branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator),
but the tuner, tuning flags, filter scheduling diagnostic and Twix optimization
monitor described here are **experimental worktree additions**, absent from the
currently tracked remote branch and possibly in progress or uncommitted remotely.
Checking out that branch alone does not provide this workflow: obtain the matching
simulator/tuner/filter implementation worktree before running the examples.

Every source path, topic, parameter and behavior below refers to that experimental
development worktree, not to main. "Production filter" means the robotics filter
used in that worktree. These instructions do not establish availability or replay
compatibility for the main robot stack. See [the simulator guide](../../docs/tooling/behavior_simulator.md)
for branch checkout instructions and [the simulator README](../simulate/README.md)
for the same availability constraints.

Uses the production filter's `Tracker` on ros-z messages decoded directly from MCAP.
It does not depend on MuJoCo or Bevy. To record and optimize automatically, use
`./simulator --tune-ball-filter logs/ball-tuning` from the repository root.

To repeat a search on fixed recordings:

```bash
cargo run -p ball-filter-tuner -- \
  --train logs/ball-tuning/train-*.mcap \
  --validation logs/ball-tuning/validation-*.mcap \
  --parameters logs/ball-tuning/baseline.json5 \
  --reference-topic simulation/ball_ground_truth_field --reference-frame field \
  --output logs/ball-tuning/another-search \
  --trials 4096 --seed 7
```

Both input arguments accept multiple files. Each file must be one complete episode
starting with a fresh filter. Each episode resets tracker state. Training and
validation must use separate recordings; do not copy the same run into both sets.
Holdout data never influences candidate selection. Output files must not already
exist. `ball_filter.json5` is a complete parameter layer and `report.json` contains
baseline/candidate metrics and parameters, search seed, budget and recording paths.
Search uses seeded bounded coordinate mutations and periodic random restarts.

The search tunes shared x/y detection noise, resting process noise, moving
position/velocity process noise, matching gate, output validity threshold,
visible/hidden decay, timeout and velocity decay. Covariance entries remain positive.
Camera timestamp tolerance and confidence thresholds stay fixed. The noise and
velocity decay parameters depend on prediction cadence; recordings should preserve
the deployment's actual odometry frequency.

## Robot recordings

The recorder must be enabled **before stack startup** and the stack must receive
a log path. It snapshots recorder settings at startup; a live parameter edit does
not start an inactive recorder. In the intended parameter layer, set
`mcap_recorder.enable: true` and configure `topics` as below. The provisioned K1
launcher in `tools/k1-setup/launch-hulk` already passes `--log-path` and maintains
`/home/booster/hulk/logs/latest`; recordings are named `recording.mcap` inside the
selected log directory. For a manually launched stack in its configured runtime,
include `--log-path /home/booster/hulk/logs/<new-episode>` along with its normal
location, parameter-root, router and `HARDWARE_ID` setup. Without a log path, the
recorder node returns without creating a recording.

Use the existing `mcap_recorder` node, with these entries in its `topics` parameter:

```text
field_dimensions
inputs/odometry
inputs/odometry/announce
camera_matrix
detected_objects
detected_objects/announce
ball_filter/update_schedule
ball_filter/ball_position
<your reference topic>
```

Capture `inputs/serial_motor_states`, `inputs/imu_state`, `inputs/camera_info`,
`support_foot`, `ground_to_robot`, and `ground_to_field` too when investigating the
kinematic chain. These are ordinary ros-z topics; there is no converted dataset or
simulator-specific filter input format. The shared filter diagnostic contains only
input timestamps, presence flags, selected camera timestamps and a cycle sequence.
It is published only when subscribed. Input payloads remain on their original topics. Announcement messages pair payload
publication IDs with sensor timestamps; payload publication time alone is not the
fusion time. Use one publisher per input topic in an episode.

Start the recorder before advancing a fresh filter. The reader rejects missing
cycles or referenced inputs and verifies replayed positions, velocities, last-seen
timestamps and missing outputs against the live filter with the supplied baseline.
Hold parameters and field dimensions fixed during an episode. Old recordings
without the scheduling diagnostic are not yet supported by this exact replay path.

The exact replay reader requires the first filter sequence to be zero and all
later cycles and referenced inputs to be present. Recorder and filter startup in
the normal robot stack is concurrent; enabling recording or restarting the stack
alone does not guarantee this startup boundary. Arrange capture subscriptions
before input/fusion processing begins and check the resulting episode with the
tuner. There is currently no robot-side CLI handshake that guarantees this ordering.
The automatic simulator capture establishes subscriptions before advancing its
logical clock and is the supported automated starting point.
Save the filter's effective baseline parameters (including location/robot overlays)
as the file supplied to `--parameters`, and keep them fixed during capture.

An independent ball reference is required to optimize absolute position error.
Publish or annotate a normal ros-z `TimeWrapper<Vec<Point3<Ground>>>` topic at the
filter output timestamps: one ball means known position, an empty vector explicitly
means absent, and a missing message means unlabelled. Multiple targets are rejected.
Use `--reference-topic <topic>` for that topic; the simulator supplies
`simulation/ball_ground_truth`. The estimated camera matrix, field localization,
and filter's own output must not be used as ground truth. For real recordings,
independently measured reference data (and its alignment to robot Ground) is still
needed; this tool does not infer it from the recording.

The existing recorder uses relative topic names. If another recorder writes fully
qualified topics, supply `--namespace /robot/name`. Wire encoding must be
`ros-z-cdr`, with the current message schemas.

With `--reference-frame ground`, the loss is a time-weighted mean of squared Ground-frame position error, with
`--penalty-metres` squared for each missed estimate or estimate while explicitly
absent (default 2 m). The report also separates position RMSE, present/absent time,
misses, false tracks and unlabelled time. A lower RMSE alone is not evidence of an
improvement. In Ground-scored runs, ground-to-field is available for diagnostics
but is not in the fitting objective.

The automatic simulator run scores in Field coordinates: the replayed ball estimate
passes through the recorded kinematic `ground_to_field`, then is compared to
`simulation/ball_ground_truth_field` (`TimeWrapper<Vec<Point3<Field>>>`). Ground-to-field
uses the absolute torso reference, not visual localization. Only preceding transforms
at most 20 ms old are accepted. Missing transforms count as missing positions while a
ball is present; false tracks in empty scenes remain penalized. The report includes
`missing_transform_seconds` so geometry coverage is visible separately from filter quality.

For robot-relative labels use `--reference-frame ground` (the CLI default) and a
`TimeWrapper<Vec<Point3<Ground>>>` reference topic. For field labels record
`ground_to_field` as well and explicitly select `--reference-frame field`. The label's
coordinate frame must match the selected option. Ground-scored reports from earlier
captures are not directly comparable with field-scored reports.
