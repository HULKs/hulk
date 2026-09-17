# Motion simulator

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

The launcher currently starts:

- `motion`: the main `motion::run_boxed` entry point. Every 20 ms of simulation
  time it dispatches the UI/behavior request to the real head and inference services,
  composes their outputs, and publishes `motion::command::RobotCommand` directly on
  `commands/robot_command`, which `hardware_interface` consumes. The motion crate
  is unchanged from `rmburg/motion-inference`; there is no dummy node or
  simulator-specific coordinator.
- `head_motion`
- `motion_inference`
- `hardware_interface`
- `global_parameter_provider`: publishes retained `joint_limits`, `player_number`,
  and `field_dimensions` from the robotics `global` parameters. The default
  `etc/parameters/location/simulator` layer matches the simulated field. Live field
  edits update the provider through the temporary parameter layer, including after
  stack resets. With `--no-robotics`, the simulator publishes field dimensions directly.

To test the head, select **Stand** and **LookAround** in the Motion command form,
click **Send command**, then **Run / Pause**. **ZeroAngles** returns the head
to zero; **LookAt** and **LookLeftAndRightOf** expose target position and height.
The game-controller form controls the field side used by head scan patterns.

**Stand** runs walking inference at zero velocity with the chosen head request.
**Walk with velocity** forwards the requested forward/lateral velocity and yaw rate.
**Kick** uses kick inference and the selected head request. Its form exposes target
speed (m/s), ball velocity (Ground, m/s), and soft/quick/strong flags. Defaults are
3.4 m/s, zero ball velocity, and all flags disabled. The soft policy ignores strong
and quick. **Stand up** exposes the fast flag, disabled by default, for full-body
get-up inference. Arm commands come directly from the main node: walking and
kicking use its configured arm controller, and get-up controls all joints.
**Damping** (also the **Damp robot** shortcut) currently sends zero commands and
gains, following upstream behavior. **Prepare** requests Booster's preparation
mode, whose RPC is not simulated.

The editor still refuses path-based **Walk** and suggests **Walk with velocity**.
External path requests use the upstream walking controller. Kick speed, ball velocity,
and policy flags are forwarded to inference, which applies its policy limits.
Target position and robot-to-field heading are still not used by kick inference.
Head and body services run concurrently using the main node's service clients.
Service errors and inference rejections retain upstream behavior, including its
current `unwrap()` calls; the simulator does not add a fallback controller.

**Look at first ball** immediately sends `Stand { head: LookAt { ... } }` for the first
spawned ball still in the scene and opens the constructed command in the form.
It continuously samples the ball's current MuJoCo center, converts it into the controlled
robot's Ground frame, and includes its height above ground with image region
Center. Moving the ball or robot updates the published target and displayed coordinates,
including while paused or dragging. **Stop tracking ball** holds the last target;
editing or sending a motion command, or pressing **Damp robot**, also stops tracking.
If the first ball is removed, tracking follows the oldest remaining ball. With no ball,
tracking stops with a message and leaves the last command unchanged.

**Kick** uses the first spawned ball's actual MuJoCo position, transformed
into the controlled robot's Ground frame. Its ball-position fields are read-only
and update live, including while paused or dragging. The sent kick keeps tracking
the ball even while editing another draft. With no ball, sending a kick is rejected;
removing the last ball stops an active kick by sending Damping. Ball velocity remains
an editable command input; it is not derived from the simulated ball's motion.

Sent commands have solid scene arrows, inspired by
[MJLab's velocity visualization](https://github.com/mujocolab/mjlab/blob/main/src/mjlab/tasks/velocity/mdp/velocity_command.py).
For **Walk with velocity**, blue shows planar velocity and green shows signed yaw
rate, both starting above the robot's torso. Length is 1 m per m/s or rad/s; a
negative yaw rate points downward. For **Kick**, an amber 1 m arrow starts
at the ball and shows `kick_direction` (a direction, not a speed or predicted path).
Directions use the robot's Ground-frame yaw and follow its current world pose.
Zero vectors are hidden. Unsent draft edits do not change the arrows; the panel
legend identifies the sent command. Arrows do not intercept ball picking or dragging.

`hardware_interface` publishes raw CDR `LowCommand` messages on `rt/joint_ctrl`.
MuJoCo applies `tau + kp * (q_target - q) + kd * (dq_target - dq)` every physics
step, clamped to each actuator's torque limits. Commands are serial and must
contain exactly 22 finite motor commands with nonnegative gains. The last valid
command is held between messages; without a command actuator torque is zero.
The model represents full custom control, so command blending weight is not used.

Mode-change RPC requests are not answered. The existing interface retries them
in a separate worker while continuing to publish joint commands. No LED commands
are generated. RPC emulation is intentionally deferred.

## External topics and time

ROS-Z topics below are relative to `--robot-namespace` (default
`/simulator/robot`). There is no `low_state_bridge` and no raw `rt/low_state`.

| Direction | Topic | Payload/source |
| --- | --- | --- |
| Publish | `inputs/low_state` | `booster::LowState`, measured MuJoCo joints and IMU |
| Publish | `camera_matrix` | `TimeWrapper<CameraMatrix>`, MuJoCo camera definition and pose |
| Publish | `ground_to_robot` | `TimeWrapper<Option<Isometry3<Ground, Robot>>>`, ground truth |
| Publish | `behavior/motion_command` | `types::motion_command::MotionCommand`, UI |
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

**Send command** or **Send game state** publishes the selected form. Edits remain drafts until sent.
The most recently sent motion command and game state are repeated every 20 ms
of simulation time, so late-starting subscribers receive them. The simulation
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

Inference consumes new tuning at request boundaries, preserving gait phase and
controller history. Model filenames, model directory, or thread-count changes
reload networks on the inference worker; service requests can temporarily time out
while loading. Failed reloads appear in the panel, retain the previous networks,
and can be corrected with another parameter update. The main node currently
unwraps rejected inference replies, so an inference fault can also stop the motion
stack and require **Reset robot & stack** after correcting the parameters.
Shared joint limits and head settings use their existing live parameter handling.

The simulator adds a temporary writable parameter layer, so UI edits survive
**Reset robot & stack** and do not alter repository configuration files. They are
discarded when the simulator closes. With `--no-robotics`, the editor contacts the
external nodes in the configured namespace and writes to their last reported layer.
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
implemented scope of this motion-only integration.

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
