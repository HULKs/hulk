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

## Robotics stack

The launcher currently starts:

- `motion_inference_dummy`: publishes a nominal `JointsCommand` on
  `motion_inference/dummy_joints` at 50 Hz of simulation time, using the Walk
  policy's configured gains. Arms use `locomotion.shoulder_roll_degrees` and
  `locomotion.elbow_degrees`, mirrored for the right arm (currently -78°/-30°
  left and +78°/+30° right); shoulder pitch/yaw stay zero. Legs start at zero
  as a fallback until walking inference responds.
- `motion`: uses the temporary `motion::run_simulator_boxed` entry point. Every
  20 ms of simulation time it extracts `MotionCommand::head_motion()` from the
  latest UI/behavior request and calls `services/head_motion`. In parallel, it
  calls `motion_inference/infer_walk` with forward/lateral/angular velocity
  exactly `(0, 0, 0)`. It merges the returned twelve leg commands and the head
  reply into the nominal pose, retaining the configured arm targets.
  It publishes the resulting `robot_command::MotionCommand`
  on `commands/motion_command` in Custom mode. A missing head request, including
  body Damping or StandUp, requests head damping. A failed service call also damps
  the head. If walking inference is unavailable or rejects a request, the legs
  fall back to the dummy's zero pose and a warning is logged. Both service calls
  have a 20 ms wall-clock timeout and cannot block each other.
- `head_motion`
- `motion_inference`
- `hardware_interface`
- `simulator_joint_limits`: publishes retained `joint_limits` from the robotics
  `global` parameters. The simulator publishes retained `field_dimensions` from
  the actual simulated field, including parameter changes and stack resets.

To test the head, select **Stand** and **LookAround** in the Motion command form,
click **Send current form**, then **Run / Pause**. **ZeroAngles** returns the head
to zero; **LookAt** and **LookLeftAndRightOf** expose target position and height.
The game-controller form controls the field side used by head scan patterns.

Walking velocity stays fixed at zero for every UI behavior request, including
walk, kick, damping, and stand-up; the real walking policy controls the legs.
The original central motion entry point and its pending body coordination remain
unchanged.

**Look at ball** immediately sends `Stand { head: LookAt { ... } }` for the first
spawned ball still in the scene and opens the constructed command in the form.
It samples the ball's current MuJoCo center, converts it into the controlled
robot's Ground frame, and includes its height above ground with image region
Center. This is a snapshot: click again to sample a moved ball. With no ball,
the button shows a message and leaves the current command unchanged.

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
| Receive, raw Zenoh | `rt/joint_ctrl` | CDR little-endian `booster::LowCommand` |

Physics uses MuJoCo's fixed timestep (currently 2 ms). Each step publishes measured
joint position, velocity, acceleration and actuator torque, plus IMU roll/pitch/yaw,
angular velocity and accelerometer readings. The serial motor list uses Booster's
joint order, mapped by MJCF names rather than MuJoCo array order. The model has no
parallel ankle motor measurements, so that list is empty. Temperature, packet loss
and reserved fields use zero defaults.

The robotics context uses `Clock::logical`. Sensor source timestamps and geometry
wrapper timestamps use the same MuJoCo time; publishing a physics frame advances
the clock and wakes robotics timers. Pause stops physics, recurring sensor
publication, and those timers. UI input can still be sent while paused.
An initial observation is published at startup and after structural model changes.

Ground is centred between the foot-link origins, projected onto the field plane,
with the robot's yaw. Camera intrinsics come from the MJCF camera's resolution and
vertical field of view; its actual pose supplies the extrinsics. MuJoCo camera
axes are converted to the projection crate's optical axes. No rendered camera
image or perception node is needed.

## Editors

The right panel has separate **Motion command** and **Game controller** forms.
Select a variant and edit its fields with number inputs and choice buttons.
Numbers support dragging or direct text entry. Angles are edited in radians,
positions in metres, and velocities in metres/second or radians/second.
Every field of the chosen MotionCommand is exposed, including nested head
requests, orientation modes, kick fields, and editable line/arc path segments.
Game state, phase, teams, time, substate, field side, and per-player penalties are
editable; the larger penalty sections can be expanded.

**Send current form** publishes that form. Edits remain drafts until sent.
The most recently sent motion command and game state are repeated every 20 ms
of simulation time, so late-starting subscribers receive them. The simulation
status shows whether a joint command has arrived and whether the stack has exited.

## Configuration

The simulator owns a local router at `tcp/127.0.0.1:7447`, with multicast discovery
disabled. To use an existing router:

```bash
./simulator --router tcp/127.0.0.1:7447
```

- `--parameter-root`: simulator parameter directory, default `tools/simulate/parameters`.
- `--robotics-parameter-root`: robotics parameter root, default `etc/parameters`.
- `--location` and `--robot`: optional location/robot parameter layers over `base`.
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
