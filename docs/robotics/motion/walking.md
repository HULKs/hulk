# Walking

## Path following and velocity commands

`MotionCommand::Walk` carries a path, desired orientation, and speed from behavior.
The motion node follows the local direction of the path, reduces speed as the remaining path shortens, and blends the walking orientation with the desired final orientation.
The result is a planar velocity in `Ground` and a yaw rate.
`MotionCommand::WalkWithVelocity` supplies the velocity and yaw rate directly.
Both commands are limited before being passed to walking inference.
[Path following](https://github.com/HULKs/hulk/blob/main/crates/nodes/motion/src/walking.rs) · [Command conversion and limits](https://github.com/HULKs/hulk/blob/main/crates/nodes/motion/src/lib.rs) · [Path planning](path_planning.md)

## Walking policy

The walking policy uses the requested velocity and robot observations to produce lower-body joint targets.
A `Stand` command uses the walking inference path with a zero velocity request.
The motion node combines the resulting body targets with head motion and publishes the resulting [robot command](overview.md#ros-z-booster-path).
[Walking policy observations](https://github.com/HULKs/hulk/blob/main/crates/nodes/motion_inference/src/locomotion/walk.rs) · [Motion node](https://github.com/HULKs/hulk/blob/main/crates/nodes/motion/src/lib.rs)
