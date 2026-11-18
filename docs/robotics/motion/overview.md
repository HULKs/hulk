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

### Motion CPU scheduling

`--motion-cpus 4,5` runs motion, low-state input, fall/pose checks, and operator-input/state nodes on a separate Tokio runtime.
Every async and blocking worker has affinity `4-5`, including workers created during scheduler handoffs.
Child threads inherit this set.
Linux chooses each thread's CPU as load changes.
The general HULK runtime and shared transport exclude these CPUs; behavior, including Voronoi and planning, stays on the general runtime.
Without the flag, all nodes share one runtime.

The Zenoh router and DDS bridge also use `CPUAffinity=4 5`.
Both CPUs remain shared with vendor processes.
No nodes or services are assigned to individual CPUs, and the vendor processes' existing affinity and priority are preserved.

`pepsi gammaray` removes CPUs 4 and 5 from kernel domain isolation and reboots when needed.
This enables scheduler load balancing; affinity alone cannot undo `isolcpus` domain isolation.
See [gammaray boot configuration](../../tooling/pepsi.md#motion-cpu-balancing).

After gammaray installs the launcher, deploy the updated binary with `pepsi upload`.
Verify `Cpus_allowed_list` is `4-5` for every motion and Zenoh thread in `/proc/<pid>/task/<tid>/status`, including after parameter binding and inference initialize.
General HULK workers should exclude CPUs 4 and 5.

The scheduler balances runnable threads.
Equal utilization and continuous migration are not required, especially when vendor threads are pinned to one CPU.
Measure motion deadlines, input freshness, and stop-response latency under perception and vendor-process load.

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
