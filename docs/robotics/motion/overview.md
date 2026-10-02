# Motion

`booster_sdk_interface` executes the `MotionCommand`
published by [behavior](../behavior/overview.md) on `behavior/motion_command`.
The launcher starts that interface and a separate `head_motion` node.

## ROS-Z Booster Path

1. **Behavior** publishes Damping, Prepare, Stand, StandUp, VisualKick, Walk,
   or WalkWithVelocity commands.
2. **SDK interface** caches the latest command and checks it on a 10 ms timer.
   Damping and Prepare request their corresponding SDK modes; the other
   commands request **Soccer** mode.
3. **Walking** converts path requests with `booster::walking::step_from_motion_command`
   and sends forward/left/turn values through the `move_robot` RPC. Stand sends
   zero velocity; WalkWithVelocity supplies velocity directly.
4. **Kicking and recovery** publish the SDK kick message and enable visual-kick
   mode, or request the SDK `get_up` operation when entering StandUp.
5. **Head motion** consumes the behavior command and sensor/gaze inputs, then
   publishes `HeadJoints<f32>` on `head_joints_command`. The SDK interface sends
   those targets through `rotate_head`; it does not call a head-motion service.

The SDK owns body execution in this path.

## Mode Requests and Recovery

The interface initializes its local `assumed_mode` to Damping and waits for a
behavior command. When the desired mode changes, it queues a retrying mode RPC
and immediately updates that assumed mode. This is not a hardware acknowledgement
gate: movement, kick, and get-up decisions use the locally assumed mode.

Mode, visual-kick, get-up, and LED requests use retry workers. Movement and head
RPCs are interval-limited. Entering StandUp starts one get-up request; a continuing
StandUp does not start a new request each tick. Leaving StandUp clears that request.

Behavior uses SDK fall state for recovery decisions. The interface sends requests
to the SDK; a locally queued or assumed mode is not proof of completed physical
recovery. Verify the actual robot state before resuming operation.

## Configuration and Inspection

Base parameters live in:

- `etc/parameters/base/booster_interface.json5`: path following, movement/kick/head
  intervals, kick power, and SDK request timeout.
- `etc/parameters/base/head_motion.json5`: head control and gaze geometry.

The base movement, kick, and head intervals are 20 ms; the SDK request timeout is
100 ms. Inspect `behavior/motion_command`, `head_joints_command`, and SDK fall
reports, together with `booster_interface::input` and `booster_interface::rpc`
logs.

## Implementation

- `crates/nodes/booster_sdk_interface/src/lib.rs`: command cache, control loop, RPC workers.
- `crates/nodes/booster_sdk_interface/src/control.rs`: mode and kick conversion.
- `crates/booster/src/walking.rs`: path-to-velocity conversion.
- `crates/nodes/head_motion/src/lib.rs`: head-target generation.

Continue with [Walking](walking.md), [Step planning and path following](step_planning.md), and [Kick and get-up requests](kicking.md).
