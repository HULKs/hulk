# Behavior and motion simulator

## Availability: experimental development checkout required

The simulator implementation and root `simulator` launcher live on
[Alex's `motion-inference-simulator` branch](https://github.com/alexschmander/hulk/tree/motion-inference-simulator),
not HULK main. This README is included as a docs-only addition; it does not add
the crate, launcher, parameter files or tests to main. For checkout instructions,
see [the simulator guide](../../docs/tooling/behavior_simulator.md#alexs-development-branch).

This document also records **experimental worktree additions**: automatic ONNX
Runtime downloading, synthetic ball perception, headless capture/replay tuning,
the standalone `ball-filter-tuner`, and the Twix optimization monitor. They are
absent from the currently tracked Alex branch and may be in progress or uncommitted
remotely. A branch checkout alone does not provide them. Use the selected branch's
README and `./simulator --help` for available behavior, and obtain the matching
implementation worktree before attempting those workflows.

All source paths, parameters, topics, tests and implementation descriptions below
refer to the development branch or those experimental worktree additions, **not
to main**. In particular, branch/worktree motion and filter behavior described
as "production" means the robotics nodes integrated in that checkout, not a
claim that these features have landed on main.

Based on `oleflb/simulator-die-zweite` (`3c6a288178f49cb9c360228823ac835aa2b06631`).
The Bevy scene runs a MuJoCo K1 and a small ROS-Z robotics stack.

## Run

From the development checkout's repository root, run (also works from fish or inside `nix develop`):

```bash
./simulator
```

The launcher sets the working directory and library paths, and forwards arguments
to the simulator. It forces the build to use the downloaded MuJoCo 3.9.0 required
by the Rust bindings, overriding system MuJoCo discovery and explicit link-directory
settings. The first build downloads it into
`${XDG_CACHE_HOME:-$HOME/.cache}/mujoco-rs`, or your existing `MUJOCO_DOWNLOAD_DIR`.

The experimental worktree launcher also resolves ONNX Runtime for `motion_inference`. An explicit
`ORT_DYLIB_PATH` takes precedence, followed by standard system library locations.
On Linux x86_64, if none is found, it downloads the official CPU ONNX Runtime 1.22.0
release, verifies its pinned SHA-256 checksum and caches it under
`${XDG_CACHE_HOME:-$HOME/.cache}/hulk-simulator/onnxruntime`. This first download
requires `curl`, `tar` and `sha256sum`. On other platforms, set `ORT_DYLIB_PATH` to
your compatible shared library. `--no-robotics` and `--help` skip this setup.
The five K1 ONNX models must be downloaded with Git LFS and are loaded from
`etc/neural_networks`.

The currently tracked branch launcher only uses an explicit `ORT_DYLIB_PATH` or
`/usr/lib/libonnxruntime.so`; it does not implement the download fallback above.

The framework runs inside the simulator process; there is no separate framework
command to start. Wait for motion-model initialization, then press **Run**.
The panel should show **Joints: connected** and **Behavior output: Stand**.
Physics stays paused while models load, on inference failure, or if a stack node
exits. Startup failures are reported in the panel and terminal. After correcting
an error, restart the simulator (or reset the stack for a recoverable node failure).
The UI requires an X11 or Wayland display and a graphics adapter supported by Bevy.

The simulator starts paused with one controlled robot. Press **Run / Pause** to
advance physics. **Reset robot & stack** pauses, returns that robot to its initial
zero-joint pose and location, clears the received command, and recreates the
robotics context and nodes. Simulation time remains monotonic across resets.
The palette can add balls and additional passive robots as physical objects.
Drag a ball from the palette to place it. Click an existing ball to select it
(highlighted gold), then hold the left mouse button and drag to reposition it.
Dragging moves the actual MuJoCo ball horizontally at its current height and
clears its linear/angular velocity. Physics pauses during the drag and resumes
on release if it was running beforehand. Moving over a sidebar holds the last
valid scene position. Click the field to clear the selection.

## Automatic ball filter tuning (experimental worktree addition)

Requires the matching simulator/tuner implementation worktree, including its
`--tune-ball-filter` flag. This flag is absent from main and the currently tracked
remote branch. From that worktree's repository root:

```bash
./simulator --tune-ball-filter logs/ball-tuning
```

Use a new output directory for each experiment. This runs without a display and
uses an isolated router, so it can run alongside the interactive simulator.
It loads the real motion stack, records six 24-second physical scenarios, verifies
that offline replay reproduces the live filter, searches 4096 parameter candidates
on four training recordings, and evaluates the best candidate on two separate
holdouts. No UI interaction is needed. `--tuning-trials` changes the search budget.

Each scenario includes standing, walking, looking away and reacquiring, a rolling
physical ball, and an empty scene with false detections. The runs use different
noise seeds, with both ball offsets and walking speeds represented in each split.
Capture rejects falls and scenarios where the robot or rolling ball did not move
sufficiently. This is an initial
regression suite, not a substitute for a larger set of recorded field conditions.

Outputs are `train-*.mcap`, `validation-*.mcap`, the effective `baseline.json5`, and
`optimized/{ball_filter.json5,report.json}`. The report includes time-weighted
position RMSE, missed-ball time, false-track time, unlabelled time and the combined
loss. A missing estimate or a track in an explicitly empty scene costs the same as
a 2-metre position error. Unlabelled frames are excluded, never treated as empty.
Validation is not used to select parameters. The report states whether it improved.
The command saves a candidate layer without modifying deployed parameters.
Automatic runs use Field-frame scoring as described under Remaining realism gaps;
older Ground-scored captures are not directly comparable.

To run the interactive simulator with that candidate:

```bash
./simulator --ball-perception --robotics-parameter-layer logs/ball-tuning/optimized
```

Recordings contain ordinary ros-z CDR messages and schemas in the existing MCAP
format. The input topics and types are the same on simulator and robot. A shared
`ball_filter/update_schedule` diagnostic records fusion batch boundaries and the
selected camera timestamps. This avoids changing the experiment through replay
scheduling. Raw joints, IMU, calibration, support-foot and ground transforms are
also captured. The optimizer reuses the recorded estimated camera matrices and leg
odometry; thus geometry errors from the actual kinematic chain remain in every trial.
It does not rerun motion or regenerate random detections for each candidate.

The production filter rejects camera matrices farther than
`ball_filter.maximum_camera_matrix_time_difference` from an image timestamp in
either direction (default 20 ms). This also applies to the image overlay. Rejected
geometry skips the measurement update; odometry prediction and track expiry still
run. The optimizer holds this tolerance fixed.

See [the tuner documentation](../ball-filter-tuner/README.md) for reusing existing
MCAP files and recording on real robots.

## Interactive ball perception (experimental worktree addition)

The `--ball-perception` flag, associated parameters and diagnostic topics require
the matching implementation worktree; they are absent from main and the currently
tracked remote branch.

Run `./simulator --ball-perception` to run the production kinematics, ground,
camera, odometry, ball-filter, and visual-kick nodes. Add physical balls
from the palette and press Run. Keep Game at Initial for stationary experiments;
Playing lets behavior react to the filtered ball. The manual Look at first ball
and Kick shortcuts still deliberately use truth, so avoid them when evaluating
closed-loop perception behavior.

There are two independent paths into the ball filter:

- MuJoCo ball centers and the **true left-camera pose** → noisy synthetic
  `detected_objects` bounding boxes.
- Measured joints / IMU → `kinematics_provider`, `support_foot_estimator` and
  `ground_provider` → `camera_matrix_calculator`, plus production leg `odometry`.

The production ball filter uses the **estimated camera matrix** to project those
boxes onto the ground, propagate covariance, associate detections and track balls.
This separation exposes errors from body wobble, support changes, kinematic model
mismatch and the nodes' asynchronous sensor pairing. It avoids cancelling geometry
errors by synthesizing detections with the same estimated camera used to decode them.
Odometry now runs at the sensor cadence, normally 500 Hz, rather than using perfect
world poses. Empty detection frames continue when no ball is visible.

Ground-to-field uses the production `localization2d` transform adapter, supplied
with the simulator's absolute 3D torso pose and the estimated `robot_to_ground`.
The torso reference retains roll, pitch, yaw and height; Home/Away rotates the
reference into team Field coordinates. **Visual localization is not launched.**
The ball filter consumes neither this absolute reference nor ground-to-field;
its inputs remain camera geometry, detections, leg odometry and field dimensions.
Thus changing field appearance, landmark matching or lighting does not change the
reference or the ball-filter tuning path. Ground-to-field is a downstream diagnostic
and a behavior input.

The true camera comes from MuJoCo's camera pose, shifted to the left optical center
by half its stereo IPD. The simulator location layer converts the model's mounting
pitch and sign convention to the production parameter's degrees. Raw joint and IMU
samples currently have no additional measurement noise or artificial delay;
physical wobble is measured and passes through the actual estimators. Production
support/fall gating remains active, so geometry may be unavailable during startup,
double support or a fall. There is no perfect-transform fallback.

Edit `ball_perception` in `tools/simulate/parameters/simulator.json5`, or live via
Twix's `/simulator/parameters` parameter node:

| Parameter | Default | Meaning |
| --- | --- | --- |
| `frame_period` | 0.04 s | Camera sampling period in simulation time, rounded up to a physics step |
| `center_noise_pixels` | 2 px | Independent Gaussian standard deviation on bounding-box center x and y |
| `false_positive_probability` | 0.05 | Chance of one extra ball box per camera frame, uniformly positioned in the image |
| `false_positive_radius` | 8 px | Radius of the false box |
| `detection_confidence` | 0.9 | Confidence of both real and false detections |
| `seed` | 42 | Repeatable detector randomness for the same sequence of camera poses and balls |

Setting noise and false-positive probability to zero gives a clean baseline.
The production filter still rejects low-confidence detections and pixels which
cannot project onto the ball-height plane. False boxes may therefore be rejected
before they become ball percepts. The model currently has no occlusion, image
rendering / neural inference, or additional missed-detection probability.
Bounding-box size uses a pinhole approximation; airborne balls use their actual
height for synthesis, while the production filter retains its ground-ball model.

Connect `./twix /simulator/robot --router tcp/127.0.0.1:7447` and inspect:

- `detected_objects`, `ball_filter/ball_percepts`, and `ball_filter/ball_filter_state`.
- `ball_filter/ball_position` for the estimate used by behavior.
- `robot_kinematics`, `support_foot`, `ground_to_robot`, `camera_matrix`,
  `inputs/odometry`, and `ground_to_field` for the production geometry outputs.
- `simulation/camera_matrix_ground_truth` and `simulation/ground_to_robot_ground_truth`
  for the corresponding perfect transforms, kept off the production input topics.
- `simulation/ball_ground_truth` for timestamped physical ball centers in Ground.
- `simulation/ball_filter_metrics`: `position_error_metres` / `position_rmse_metres`
  are the primary, localization-independent Ground-frame metrics.
  `field_position_error_metres` / `field_position_rmse_metres` separately measure
  error after estimated ground-to-field conversion. Field metrics use the latest
  transform at or before the estimate's source time, reject transforms older than
  100 ms, and expose their age and missing-transform count.

Metrics compare against exact historical truth at the filter state's timestamp;
processing latency is excluded. Missing estimates and estimates without any real
ball are counted separately. With multiple balls, error is distance to the nearest
real ball, so use one ball for unambiguous tuning.

Tune production parameters through `/simulator/robot/ball_filter` in Twix. Compare
RMSE together with missing/false-track counts; RMSE alone rewards withholding
estimates. Counters are per emitted state, not time-weighted. Reset robot & stack
clears filter state, metrics and the random stream. Reset between parameter
comparisons; live edits otherwise mix results in the accumulated metrics. This
provides live inspection; use the automatic recording/replay workflow above for
parameter optimization.

Pausing stops observation sampling and random draws. Resuming preserves state;
resetting the stack retains monotonic simulation time. `--no-robotics` can be
combined with `--ball-perception` to publish measured sensors, camera calibration,
absolute torso reference and detections to an external geometry/filter stack.
The default invocation continues using perfect ball inputs.

## Robotics stack

The launcher starts the production `behavior_node`, `ball_state_composer`,
`rule_obstacle_composer`, `fall_detection`, `motion`, `head_motion`,
`motion_inference`, `hardware_interface`, and `global_parameter_provider` nodes.
They share MuJoCo's logical clock. Behavior ticks every 20 ms and owns
`behavior/motion_command`; motion sends the resulting joint commands through the
real hardware interface. The simulator supplies the inputs listed below.

The global provider publishes retained `joint_limits`, `player_number`, and
`field_dimensions`. Live field changes update its temporary parameter layer.
With `--no-robotics`, the simulator publishes field dimensions directly instead.

The simulator starts paused, with **no injected motion command**. Its temporary
layer clears the base configuration's injection and disables remote control.
The default Game state is Initial, so behavior requests Stand with a head scan.
Set Game to Playing and send it to enable ball pursuit and kicking; add a ball
from the palette. With no ball, behavior searches after its last-ball timeout.

**Inject command** writes the form to the behavior node's
`control.injected_motion_command` parameter. **Clear injected motion — let behavior
control** writes `null` and stops UI ball/kick tracking. Clearing leaves the draft
available for reuse and lets behavior follow the current Game settings on its
next logical tick. Parameter writes work while paused; behavior output updates
when simulation resumes. Injection follows the production tree's priorities:
Stop overrides it, and remotely enabling the behavior remote-control mode also
takes precedence. Normal motion safety checks still apply to injected commands.

To test the head, select **Stand** and **LookAround** in the Motion command form,
click **Inject command**, then **Run / Pause**. **Space** toggles play/pause unless you are editing a text
field or dragging a ball. **ZeroAngles** returns the head
to zero; **LookAt** and **LookLeftAndRightOf** expose target position and height.
The game-controller form controls the field side used by head scan patterns.

**Stand** runs walking inference at zero velocity with the chosen head request.
**Walk with velocity** forwards the requested forward/lateral velocity and yaw rate.
**Kick** uses kick inference and the selected head request. Its form exposes target
speed (m/s) and soft/quick/strong flags, plus live ball and target readouts. Defaults
are 3.4 m/s and all flags disabled. The soft policy ignores strong
and quick. **Stand up** exposes the fast flag, disabled by default, for full-body
get-up inference. Arm commands come directly from the main node: walking and
kicking use its configured arm controller, and get-up controls all joints.
**Damping** (also the **Damp robot** shortcut) currently sends zero commands and
gains, following upstream behavior. **Prepare** requests Booster's preparation
mode. The simulator acknowledges this mode but does not implement Booster's preparation pose controller.

The editor still refuses path-based **Walk** and suggests **Walk with velocity**.
External path requests use the upstream walking controller. Kick speed, ball velocity,
and policy flags are forwarded to inference, which applies its policy limits.
Head and body services run concurrently using the main node's service clients.
Service errors, stale inputs, and recovery transitions use the upstream motion
safety lifecycle. A latched control fault needs Damping followed by Prepare,
then the desired command, or **Reset robot & stack**. Fall detection runs on
measured simulated joints/IMU; recovery completion is not faked.

**Look at first ball** immediately sends `Stand { head: LookAt { ... } }` for the first
spawned ball still in the scene and opens the constructed command in the form.
It continuously samples the ball's current MuJoCo center, converts it into the controlled
robot's Ground frame, and includes its height above ground with image region
Center. Moving the ball or robot updates the published target and displayed coordinates,
including while paused or dragging. **Stop tracking ball** holds the last target;
editing or sending a motion command, or pressing **Damp robot**, also stops tracking.
If the first ball is removed, tracking follows the oldest remaining ball. With no ball,
tracking stops with a message and leaves the last command unchanged.

**Kick** uses the first spawned ball's actual MuJoCo position and linear velocity,
transformed into the controlled robot's Ground frame. Kick direction automatically
aims from the ball toward the center of the right goal (field +X goal line).
These fields are read-only and update live as the ball or robot moves, including
while paused or dragging; resizing the field updates the aim too. Velocity is
measured in m/s, with world motion rotated into Ground axes. Dragging resets ball
velocity to zero. The sent kick keeps tracking even while editing another draft.
With no ball, sending a kick is rejected; removing the last ball stops an active
kick by sending Damping.

Behavior output commands have solid scene arrows, inspired by
[MJLab's velocity visualization](https://github.com/mujocolab/mjlab/blob/main/src/mjlab/tasks/velocity/mdp/velocity_command.py).
For **Walk with velocity**, blue shows planar velocity and green shows signed yaw
rate, both starting above the robot's torso. Length is 1 m per m/s or rad/s; a
negative yaw rate points downward. For **Kick**, an amber 1 m arrow starts
at the ball and shows `kick_direction` (a direction, not a speed or predicted path).
Directions use the robot's Ground-frame yaw and follow its current world pose.
Zero vectors are hidden. Unsent draft edits do not change the arrows; the panel
legend identifies the active command, including autonomous output. Arrows do not intercept ball picking or dragging.

`hardware_interface` publishes raw CDR `LowCommand` messages on `rt/joint_ctrl`.
MuJoCo applies `tau + kp * (q_target - q) + kd * (dq_target - dq)` every physics
step, clamped to each actuator's torque limits. Commands are serial and must
contain exactly 22 finite motor commands with nonnegative gains. The last valid
command is held between messages; without a command actuator torque is zero.
The model represents full custom control, so command blending weight is not used.

A small raw SDK responder acknowledges Damping, Prepare, and Custom mode changes
on `rt/LocoApiTopicReq` / `rt/LocoApiTopicResp`. This lets the real actuator enforce
its mode-acknowledgement contract. Other SDK actions are rejected. Prepare has no
simulated SDK pose controller, and LEDs are not modeled. Raw SDK and joint topics
are unnamespaced: use a dedicated router, with one controlled robot per router.

## Behavior input map

All topic names below are relative to `/simulator/robot` by default. This is a
single controlled robot with perfect state substitutions by default.
`--ball-perception` replaces the ball and ground-to-field inputs and runs production
geometry as described above. The table below describes the default mode.
MuJoCo world +X/+Y is converted into the robot's yaw-aligned Ground frame. Field
coordinates use the team convention: Home is world-aligned, Away rotates by π,
so autonomous behavior always attacks Field +X. The UI's manual kick shortcut
continues to aim toward world +X, independently of team side.

| Behavior input | Supplied by | Functionality and limits |
| --- | --- | --- |
| `field_dimensions` | Real global provider, synchronized to simulator field | Field geometry, kickoff poses, goals, and rule geometry follow field edits. |
| `player_number` | Real global provider (`global.player_number`) | Correct player penalty and configured role; editable through Twix. |
| `primary_state` | UI filtered game state mapped directly, including this player's penalty | Initial/Ready/Set/Playing/Stop/Penalized/Finished work. Physical button arming and primary-state-filter transitions are bypassed. Damping/Prepare are available as motion injections. |
| `filtered_game_controller_state` | Existing Game form | Match phase, kickoff, penalties and set plays can be exercised manually; no referee, whistle detection, countdown, or automatic match progression. |
| `ground_to_field` | MuJoCo foot midpoint and torso yaw, with team-side rotation | Localization-dependent walking and aiming work perfectly; no localization drift, ambiguity, or relocalization tests. |
| `ball_state` | Real ball composer from ground-truth `ball_filter/ball_position` | Oldest remaining ball, measured position/velocity, simulation-time last-seen stamp. Pursuit, interception, and kicking work; no visibility, occlusion, camera noise, false detections, or team-ball fusion. No balls publishes `None`. |
| `visual_kick/ball_position` | Same ground-truth ball in Ground coordinates | Fresh kick inputs with simulation timestamps; visual-kick tracking failures and latency are absent. |
| `rule_ball_state` | Real ball composer from UI game state, pose, dimensions, and primary state | Kickoff/penalty rule-ball placement follows production logic. |
| `rule_obstacles` | Real rule obstacle composer | Kickoff, opponent free-kick, and penalty restrictions follow production logic and manually supplied game state. |
| `obstacles` | Ground-truth passive robot torso positions, conservative radii | Robot avoidance can be exercised; passive robots have physics but no decisions. Goal structures remain physical collisions and are not supplied as planner obstacles. No detection noise or classification tests. |
| `position_of_interest` | Ball position, otherwise one metre straight ahead | Deterministic gaze fallback, without a tactical attention model. |
| `fall_detection/status` | Real fall detector from MuJoCo `inputs/low_state` | Measured falling/fallen/upright classification and readiness; thresholds and dynamics remain those of the production node and simulated robot. |
| `motion/execution` | Real motion node | Actual recovery phase, completion, and fault feedback; no fabricated successful get-up. |
| `player_states` | Absent; behavior defaults to all players absent | Behavior selects its last-player striker/search branch. Cooperative role allocation, supporter positioning, Voronoi ownership contests, teammate passing, and the ordinary goalkeeper branch are not exercised. Passive robots do not count as teammates. Requires simulated team identities and independent stacks/state messages. |
| `hypothetical_ball_positions` | Absent; empty default | No uncertain-ball gaze candidates. Ground truth cannot produce meaningful perception hypotheses without an observation model. |
| `suggested_search_position` | Absent; `None` default | No distributed search suggestion. Current search subtree already uses its turning search action; the suggested-position walking branch is commented out upstream. |
| `game_controller_address` | Absent; `None` default | No return packets to a real GameController. Behavior can emit team messages on `outputs/message`, but no network node transmits or routes them. |
| `behavior_node` parameters | Base/location/robot layers plus temporary override layer | Full production strategy settings, remotely editable with Twix. Injection and remote control start cleared/disabled. |

The most useful next additions are team identities/message routing for cooperative
behavior, and rendered camera perception and visual localization. Use
`--ball-perception` for the existing synthetic visibility/noise model and production
ball filtering. The default ground-truth mode alone does not validate perception loss
or cooperative behavior.

## Connect Twix

Yes. Start the simulator, then run ROS-Z Twix from another terminal:

```bash
./twix /simulator/robot --router tcp/127.0.0.1:7447
```

Use the namespace passed to `--robot-namespace` and the same router endpoint if
you override either. The default router listens on loopback; for another machine,
run a shared reachable router and pass its endpoint to both programs.

Useful Text topics are `behavior/motion_command`, `behavior/blackboard`,
`behavior/trace`, `fall_detection/status`, `motion/execution`,
`motion_inference/status`, `hardware_interface/status`, `ball_state`,
`ground_to_field`, and `rule_obstacles`. The Parameter panel can edit
`/simulator/robot/behavior_node`, including `control.injected_motion_command`
(`null` returns control), and the other running nodes. Use namespace `/simulator`
for the simulator's own `parameters` node. There are no camera image topics, so
an Image panel will not receive rendered vision. Behavior and motion outputs stop while paused; parameter services and scene
state updates remain available. The Map panel's Field, Robot Pose, Ball Position,
Obstacles, Path, and Path Obstacles layers use topics provided by this stack.
Ball-filter debug layers have source nodes with `--ball-perception`; localization-debug layers do not.

## External topics and time

ROS-Z topics below are relative to `--robot-namespace` (default
`/simulator/robot`). There is no `low_state_bridge` and no raw `rt/low_state`.

| Direction | Topic | Payload/source |
| --- | --- | --- |
| Publish | `inputs/low_state` | `booster::LowState`, measured MuJoCo joints and IMU |
| Publish, default mode | `camera_matrix` | `TimeWrapper<CameraMatrix>`, true MuJoCo left-camera pose |
| Publish, default mode | `ground_to_robot` | `TimeWrapper<Option<Isometry3<Ground, Robot>>>`, ground truth |
| Publish, perception mode | `inputs/serial_motor_states`, `inputs/imu_state` | Measured joints and IMU, with original simulation timestamps |
| Publish, perception mode | `inputs/camera_info` | Left-camera intrinsics and image dimensions |
| Publish, perception mode | `localization/pose_3d` | Absolute torso reference; only the 2D transform adapter consumes it |
| Observe, perception mode | `camera_matrix`, `ground_to_robot`, `inputs/odometry`, `ground_to_field` | Computed by production nodes |
| Observe | `behavior/motion_command` | `MotionCommand`, emitted only by behavior |
| Publish | `filtered_game_controller_state` | `FilteredGameControllerState`, UI |
| Publish | `field_dimensions` | `FieldDimensions`, actual simulator parameters, retained |
| Publish | `joint_limits` | `JointLimits`, robotics global parameters, retained |
| Publish | `player_number` | `PlayerNumber`, robotics global parameters, retained |
| Receive, raw Zenoh | `rt/joint_ctrl` | CDR little-endian `booster::LowCommand` |

Physics uses MuJoCo's fixed timestep (currently 2 ms). Each step publishes measured
joint position, velocity, acceleration and actuator torque, plus IMU roll/pitch/yaw,
angular velocity and accelerometer readings. The serial motor list uses Booster's
joint order, mapped by MJCF names rather than MuJoCo array order. The model has no
parallel ankle motor measurements, so that list is empty. Temperature, packet loss
and reserved fields use zero defaults.

The robotics context uses `Clock::logical`. Before publishing each physics frame,
the simulator advances the shared clock to that frame's MuJoCo time, also used for
sensor source timestamps and geometry wrappers. This prevents concurrent service
requests from seeing observations ahead of their clock. Advancing time wakes robotics
timers. Pause stops physics, recurring sensor
publication, and those timers. UI input can still be sent while paused.
An initial observation is published at startup and after structural model changes.

Ground is centred between the foot-link origins, projected onto the field plane,
with the robot's yaw. Camera intrinsics come from the MJCF camera's resolution and
vertical field of view; its actual pose supplies the true left-camera extrinsics.
MuJoCo camera axes are converted to the projection crate's optical axes. In
perception mode these extrinsics are used only for detection synthesis and debug;
the production camera matrix comes from joint/IMU estimates. No rendered image is needed.

## Editors

The right panel has **Commands**, **Game**, and **Parameters** tabs.
Select a variant and edit its fields with number inputs and choice buttons.
Numbers support dragging or direct text entry. Angles are edited in radians,
positions in metres, and velocities in metres/second or radians/second.
Every field of the chosen MotionCommand is exposed, including nested head
requests, orientation modes, kick fields, and editable line/arc path segments.
Game state, phase, teams, time, substate, field side, and per-player penalties are
editable; the larger penalty sections can be expanded.

**Inject command** applies the motion override; **Send game state** publishes the
selected match settings. Edits remain drafts until sent. The game state is repeated
every 20 ms of simulation time. The motion override is a persistent node parameter
and is only written when changed, so periodic publication does not overwrite Twix edits. The simulation
status shows whether a joint command has arrived and whether the stack has exited.

The **Parameters** tab exposes every field of `head_motion`, `motion_inference`
(including all five policies), and `hardware_interface`, plus the shared global
joint limits. Use the pinned Head / Inference / Hardware / Joint limits selector;
expand section headers for nested settings. Related numeric fields are paired,
model paths have text inputs, and the injected head position has an enable button.
Durations use seconds; joint angles use radians unless named in degrees.

**Apply live** sends the selected group's fields as one atomic ROS-Z parameter
transaction to its running node. It does not pause physics, reset the robot,
restart nodes, or reset simulation time. The footer reports pending changes,
service availability, rejection reasons, and completion. Drafts in other groups
are retained and marked with an asterisk. **Discard edits** reloads the selected
form from the latest node snapshot. Node snapshots refresh in the background;
external edits are reflected when the form has no unsent changes. Revision checks
reject stale drafts instead of overwriting another editor's changes.

The rebased inference node rejects live inference parameter changes: edit its
configuration before starting a new simulator process. The UI reports that rejection;
a stack reset alone does not apply a rejected draft. Hardware also rejects changes
to its actuator output period without restart. Shared joint limits and head settings
retain their live parameter handling.

The simulator adds a temporary writable parameter layer, so UI edits survive
**Reset robot & stack** and do not alter repository configuration files. They are
discarded when the simulator closes. Reset also reapplies the current UI override
selection, including a pending clear. With `--no-robotics`, the editor contacts the
external nodes in the configured namespace and writes to their last reported layer.
An external behavior node needs a lower layer with
`control.injected_motion_command: null` and a separate writable top layer; otherwise
recursive merging can combine the base injection's enum variant with a UI variant.
The built-in stack creates these two temporary layers automatically.
Field geometry and ball physics remain available through the simulator parameter
service below. Arm handling remains owned by the upstream motion node; inference
arm-angle edits do not override its current zero arm commands during walking or kicking.

The right panel uses a fixed simulation toolbar, selected tabs, a scrolling form,
and a fixed feedback/action area. Its visual pass follows
[Anthropic's frontend-design skill](https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md)
with Fira Sans labels, slate surfaces, blue active controls, and amber errors.

## Configuration

The simulator owns a local router at `tcp/127.0.0.1:7447`, with multicast discovery
disabled. To use an existing router:

```bash
./simulator --router tcp/127.0.0.1:7447
```

- `--parameter-root`: simulator parameter directory, default `tools/simulate/parameters`.
- `--robotics-parameter-root`: robotics parameter root, default `etc/parameters`.
- `--location`: location layer over `base`, default `simulator`.
- `--robot`: optional robot parameter layer over the location layer.
  Simulator-local defaults (including `hardware_interface`) are loaded first, so
  robotics base/location/robot layers can override them.
- `--robot-namespace`: namespace shared by the robotics nodes and UI publishers.
- `--no-robotics`: run the UI, sensors and raw command receiver without launching nodes;
  useful for testing publishers or running the stack externally.

The simulator parameters remain available on `/simulator/parameters`:

```bash
rosz parameter snapshot --node /simulator/parameters
rosz parameter set field_dimensions.length 10.0 \
  --node /simulator/parameters --layer <layer-reported-by-snapshot>
```

The imported `USERSTORIES.md` describes broader prototype ambitions, not a list of
released features. This README records branch ground-truth behavior and experimental
worktree synthetic ball-perception and headless recording/replay tuning workflows.

## Checks

For direct Cargo checks, configure MuJoCo in your shell first (Bash syntax):

```bash
export MUJOCO_NO_PKG_CONFIG=1
export MUJOCO_DOWNLOAD_DIR="${MUJOCO_DOWNLOAD_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/mujoco-rs}"
export LD_LIBRARY_PATH="$MUJOCO_DOWNLOAD_DIR/mujoco-3.9.0/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
cargo check -p simulate
cargo test -p simulate
```

Tests cover robot joint mapping, PD and torque limits, camera geometry, ground
coordinates, form variants, ROS-Z message/source-time delivery and raw CDR control,
as well as the prototype's scene and model-recompilation checks.

## Startup regression test

With the MuJoCo library on `LD_LIBRARY_PATH`, `ORT_DYLIB_PATH` set, and the K1
models downloaded, run the headless production-stack test:

```bash
cargo test -p simulate real_stack_drives_simulated_robot_after_startup -- --ignored --nocapture
```

It loads inference while simulation time is paused, then checks physical head
movement, forward displacement from an injected walk, and behavior takeover after
clearing the injection. No display is needed. The test is opt-in because it needs
the native runtimes and model files; the ordinary motion tests cover paused-time
status updates and source-timestamp freshness without loading ONNX models.

### Watch optimization in Twix (experimental worktree addition)

Requires matching simulator, tuner and Twix worktree source. Neither main nor the
currently tracked Alex branch includes this panel or `--keep-tuning-open` flag.

Start a recording and optimization run with:

```sh
./simulator --tune-ball-filter logs/my-ball-run --keep-tuning-open
./twix
```

In Twix, use the **+** menu to add **Ball-filter optimization**, then click
**Connect to simulator / optimizer**. The panel connects directly to the local
headless simulator/optimizer; no router or namespace entry is needed. It has its
own connection, leaving other panels' robot connections unchanged. The optimizer
owns `tcp/127.0.0.1:7448` for the entire run, including changes between recordings.
Only one local optimization run can use that endpoint at a time.

The panel shows recording/scenario progress, the true and filtered ball relative
to the robot during capture, trial progress, the best training loss, and baseline
versus candidate position RMSE, missing-ball time, and false-track time. Held-out
scores appear after the search. The graph contains updates received since
connecting; the latest scores and parameters are available when connecting late.
The live ball view shows the baseline filter during capture; optimization uses
recorded messages and does not animate each candidate. `--keep-tuning-open` keeps
the final result available until Ctrl-C. A stopped publisher is explicitly shown
as stale in Twix.

### Remaining realism gaps

The current scenarios exercise physical walking/wobble, head motion, rolling balls,
Gaussian pixel noise and occasional false detections. They are a starting point,
not a calibrated model of real robot errors. Prioritize these extensions:

- Sensor and image timing: exposure timestamps, encoder/IMU sampling offsets,
  delivery jitter, latency, dropped frames and out-of-order delivery. The ball
  filter's 20 ms camera freshness bound rejects old outer timestamps, but a fresh
  camera matrix can still contain stale joints selected upstream by nearest-time
  lookup. Synchronization needs to be checked at each geometry input, with separate
  semantics for retained state such as support-foot changes.
- Sensor errors: encoder quantization/zero offsets/backlash, IMU noise and slowly
  varying bias, and attitude-estimator lag or errors during acceleration. Current
  joint measurements and IMU attitude are ideal MuJoCo measurements.
- Calibration errors: perturb true versus assumed camera mounting, intrinsics,
  joint zero offsets and link lengths. Include fixed biases per recording and
  slow drift, not only independent white noise. Keep truth/reference poses clean.
- Detection errors over time: burst dropouts, motion blur, occlusion, bounding-box
  size/confidence errors and persistent distractors. Independent false boxes do
  not model a field marking repeatedly misclassified as a ball.
- Contact/model errors: turf friction, slipping feet, compliance and load-dependent
  flex, ball spin/bounces and variable rolling resistance.

Two existing filter assumptions also deserve separate investigation before
real-robot deployment: the field-boundary check currently compares Ground-frame
coordinates to field extents, and the resting/moving switch reads the position
covariance block for its velocity test. Changing filter behavior requires a new
baseline capture for the replay-parity check; these are not tuned away by noise.

Automatic runs score the final position in Field coordinates, applying the recorded
kinematic ground-to-field transform before comparison with clean simulated field
truth. Twix reports the scoring frame and missing-transform time. The small live
capture view remains robot-relative; it is a preview of the baseline filter, not
the offline score.
