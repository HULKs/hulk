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

The ROS-Z Booster stack bypasses the legacy `commands_sender` path. Behavior publishes `behavior/motion_command`, the ROS-Z head nodes publish `head_joints_command`, and `booster_interface` owns Booster SDK mode changes, walking commands, head rotation, stand-up requests, LED forwarding, local emergency damping, and `rt/kick_ball` publishing.

`booster_interface` reads its runtime parameters from `etc/parameters/ros_z/base/booster_interface.json5`. The removed split ROS-Z nodes no longer consume `commands/high_level_command`, `services/get_robot_mode`, or `command_sender` parameters. Robot mode is now managed internally from `behavior/motion_command` and the latest SDK mode poll.

Emergency damping uses a local toggle and the configured remote toggle. F1 or stand short press flips the local toggle; the robot damps while `local_stop_toggle != booster_interface.remote_stop_toggle`. Keep `remote_stop_toggle` aligned with the external remote-stop state when starting or recovering the robot.

Manual validation on a Booster robot should check these behaviors:

- F1 or stand short press toggles local emergency damping.
- Changing `booster_interface.remote_stop_toggle` changes emergency damping according to the XOR rule above.
- Walking commands produce Booster SDK `move_robot` calls only after the SDK reports `Walking` mode.
- `head_joints_command` produces Booster SDK `rotate_head` calls when walking is allowed.
- Stand-up commands produce Booster SDK `get_up` requests in `Prepare` or `Damping` mode and are suppressed while emergency damping is active.
- Visual kick commands start Booster SDK `visual_kick(true)` while walking is allowed.
- Visual kick commands stop Booster SDK `visual_kick(false)` when the command ends or walking becomes disallowed.
- Visual kick commands publish `rt/kick_ball` while walking is allowed.
- Unknown SDK robot mode ids produce a log entry containing the raw id and keep walking disabled.
