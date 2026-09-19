# Behavior and motion simulator

Based on `oleflb/simulator-die-zweite` (`3c6a288178f49cb9c360228823ac835aa2b06631`).
The Bevy scene runs a MuJoCo K1 and a small ROS-Z robotics stack.

## Run

From the repository root, run (also works from fish or inside `nix develop`):

```bash
./simulator
```

The launcher sets the working directory and library paths, and forwards arguments
to the simulator. It forces the build to use the downloaded MuJoCo 3.9.0 required
by the Rust bindings, overriding system MuJoCo discovery and explicit link-directory
settings. The first build downloads it into
`${XDG_CACHE_HOME:-$HOME/.cache}/mujoco-rs`, or your existing `MUJOCO_DOWNLOAD_DIR`.

The launched `motion_inference` node also needs ONNX Runtime (the existing node
uses `ort` with dynamic loading). The launcher uses `/usr/lib/libonnxruntime.so`
when available; an explicit `ORT_DYLIB_PATH` takes precedence. For other locations,
set that variable to your ONNX Runtime shared library or put it on the library
search path. The five K1 ONNX models must be downloaded with Git LFS and are loaded
from `etc/neural_networks`.
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
single controlled robot with perfect state substitutions, not a perception test.
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
behavior, and a visibility/noise model for ball loss and uncertain perception.
Ground truth alone does not validate those parts of the behavior tree.

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
Perception-filter and localization-debug layers have no source nodes here.

## External topics and time

ROS-Z topics below are relative to `--robot-namespace` (default
`/simulator/robot`). There is no `low_state_bridge` and no raw `rt/low_state`.

| Direction | Topic | Payload/source |
| --- | --- | --- |
| Publish | `inputs/low_state` | `booster::LowState`, measured MuJoCo joints and IMU |
| Publish | `camera_matrix` | `TimeWrapper<CameraMatrix>`, MuJoCo camera definition and pose |
| Publish | `ground_to_robot` | `TimeWrapper<Option<Isometry3<Ground, Robot>>>`, ground truth |
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
vertical field of view; its actual pose supplies the extrinsics. MuJoCo camera
axes are converted to the projection crate's optical axes. No rendered camera
image or perception node is needed.

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

The imported `USERSTORIES.md` describes broader prototype ambitions, not the
implemented scope of this ground-truth behavior/motion integration.

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
