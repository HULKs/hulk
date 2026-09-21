# Overview

!!! note

    This structure in three steps is only conceptionally, in the code, there is no differentiation between these steps, and all nodes are automatically sorted during compile time depending on their inputs and outputs.

## Motion Selection

Motion starts in the `motion_selector` with the motion command from behavior.
Here the current motion is chosen based on the previous motion, if it is finished or if it can be aborted.

## Motion Execution

In the next step, all nodes for all motions are executed.
The nodes, whose motion is not selected, may exit early.

## Command Sending

Motion finishes by collecting and optimizing all motor commands in the `motor_commands_collector`, then writes them to the hardware interface in the `commands_sender`.

## ROS-Z Booster Path

### CPU isolation

`hulk_ros_z --motion-cpus 4,5` places `motion`, `motion_inference`, `head_motion`,
and `hardware_interface` on a separate Tokio runtime with one async worker per
selected Linux CPU (CPU numbering starts at zero). Each worker is pinned to one
CPU; the two workers execute tasks across CPUs 4 and 5. Their nested async tasks
use the same runtime. Additional blocking workers are pinned round-robin across
the selected CPUs, and native threads they create inherit that worker's affinity.
`motion_inference.inference_threads` remains at its default of 1.

Individual thread pinning matters because the robot's `isolcpus=4-5` boot setting
disables normal load balancing on those cores. A probe inside the HULK container
confirmed that threads with the shared affinity mask `4-5` could all remain on
CPU 4. Pinning workers individually ensures that both cores are used.

The main thread excludes the selected CPUs before starting the general runtime
and shared transport, so their threads inherit the remaining allowed CPUs.
Both runtimes retain node failure monitoring and bounded shutdown. An unavailable
CPU, or an affinity mask with no CPU left for the general runtime, fails startup.
Without `--motion-cpus`, all nodes use the shared runtime as before; this is also
the default for development and simulation.

The K1 launcher in `tools/k1-setup/launch-hulk` passes `--motion-cpus 4,5`. Existing
robots need both the new binary and updated `/usr/bin/launch-hulk`; uploading
only the binary does not enable isolation. Robot 43 was inspected with
`pepsi shell 43`: its kernel uses `isolcpus=4-5`. A five-second sample measured
12.1% busy on CPU 4 and 34.4% on CPU 5. The vendor hardware threads run on CPU 4;
the vendor motion loops run on CPU 5. This sample does not establish the available
budget across all motion states.

This separates motion from HULK's other executor work, while still sharing CPUs
with vendor processes and sharing Zenoh transport, memory, and upstream sensor
processing with the rest of HULK. It does not reserve an exclusive CPU or change
real-time priorities. After deployment, verify `Cpus_allowed_list: 4` or `5` for
each `hulk-motion` thread in `/proc/<pid>/task/<tid>/status`, with async workers on
both CPUs, and measure motion timing
and missed deadlines under perception load while walking and kicking.

### Command path

The ROS-Z Booster stack bypasses the legacy `commands_sender` path. Behavior publishes `behavior/motion_command`, and `hardware_interface` owns Booster Zenoh RPC mode changes, walking commands, head rotation, stand-up requests, LED forwarding, and `rt/kick_ball` publishing.

Head motion runs in `crates/nodes/head_motion`.
The launcher starts its `services/head_motion` service, which accepts a `HeadMotion` request and returns `HeadJoints<MotorCommand>`.
Connecting the service caller in central motion remains pending.

`hardware_interface` reads its runtime parameters from `etc/parameters/base/hardware_interface.json5`.
The removed split ROS-Z nodes no longer consume `commands/high_level_command`, `services/get_robot_mode`, or `command_sender` parameters.
Robot mode is now managed internally from `behavior/motion_command` without waiting for SDK mode feedback.

Manual validation on a Booster robot should check these behaviors:

- Before the first `behavior/motion_command` arrives, `hardware_interface` does not send Booster Zenoh RPC motion requests.
- Mode changes send one Booster Zenoh RPC `change_mode` request when the locally desired motion mode changes.
- `Damping` commands request Booster Zenoh RPC `Damping` mode.
- `Prepare` and stand-up commands request Booster Zenoh RPC `Prepare` mode.
- `Stand`, `Walk`, `WalkWithVelocity`, and `Kick` commands request Booster Zenoh RPC `Soccer` mode, not Booster Zenoh RPC `Walking` mode.
- Walking commands produce periodic Booster Zenoh RPC `move_robot` calls at about `50 Hz` while locally assuming `Soccer`.
- `Stand` commands produce periodic zero-velocity Booster Zenoh RPC `move_robot` calls at about `50 Hz` while locally assuming `Soccer`.
- Stand-up commands produce one Booster Zenoh RPC `get_up` request on entry while locally assuming `Prepare`.
- Visual kick commands publish `rt/kick_ball` and send one Booster Zenoh RPC `visual_kick(true)` request on visual-kick entry while locally assuming `Soccer`.
- Visual kick commands keep publishing fresh `rt/kick_ball` at about `50 Hz` while active.
- Leaving visual kick for `Stand`, `Walk`, or `WalkWithVelocity` sends one Booster Zenoh RPC `visual_kick(false)` request while staying in locally assumed `Soccer` mode.
- Robot logs contain behavior input, button input, primary-state, RPC action schedule, and RPC action completion entries for transition debugging.
