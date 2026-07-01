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

The ROS-Z Booster stack bypasses the legacy `commands_sender` path. Behavior publishes `behavior/motion_command`, the ROS-Z head nodes publish `head_joints_command`, and `booster_interface` owns Booster SDK mode changes, walking commands, head rotation, stand-up requests, LED forwarding, and `rt/kick_ball` publishing.

`booster_interface` reads its runtime parameters from `etc/parameters/ros_z/base/booster_interface.json5`. The removed split ROS-Z nodes no longer consume `commands/high_level_command`, `services/get_robot_mode`, or `command_sender` parameters. Robot mode is now managed internally from `behavior/motion_command` without waiting for SDK mode feedback.

Manual validation on a Booster robot should check these behaviors:

- Before the first `behavior/motion_command` arrives, `booster_interface` does not send Booster SDK motion requests.
- Mode changes send one Booster SDK `change_mode` request when the locally desired motion mode changes.
- `Damping` commands request Booster SDK `Damping` mode.
- `Prepare` and stand-up commands request Booster SDK `Prepare` mode.
- `Stand`, `Walk`, `WalkWithVelocity`, and `VisualKick` commands request Booster SDK `Soccer` mode, not Booster SDK `Walking` mode.
- Walking commands produce periodic Booster SDK `move_robot` calls at about `50 Hz` while locally assuming `Soccer`.
- `Stand` commands produce periodic zero-velocity Booster SDK `move_robot` calls at about `50 Hz` while locally assuming `Soccer`.
- `head_joints_command` produces periodic Booster SDK `rotate_head` calls at about `50 Hz` while locally assuming `Soccer`.
- Stand-up commands produce one Booster SDK `get_up` request on entry while locally assuming `Prepare`.
- Visual kick commands publish `rt/kick_ball` and send one Booster SDK `visual_kick(true)` request on visual-kick entry while locally assuming `Soccer`.
- Visual kick commands keep publishing fresh `rt/kick_ball` at about `50 Hz` while active.
- Leaving visual kick for `Stand`, `Walk`, or `WalkWithVelocity` sends one Booster SDK `visual_kick(false)` request while staying in locally assumed `Soccer` mode.
- Robot logs contain behavior input, button input, primary-state, SDK action schedule, and SDK action completion entries for transition debugging.
