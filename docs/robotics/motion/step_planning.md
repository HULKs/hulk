# Step Planning and Path Following

On main `0711900e0`, the SDK interface follows the path supplied in
`MotionCommand::Walk`; it does not plan a sequence of individual footsteps.
Behavior chooses destinations and builds walking paths, while
`booster::walking::step_from_motion_command` converts a path into linear and
angular velocity for the Booster SDK `move_robot` RPC.

The conversion uses path direction and length, requested speed, orientation mode, target orientation, and alignment/deceleration parameters.
Although it returns `types::step::Step`, its forward/left/turn values become SDK velocity inputs.
See [Walking](walking.md) for the calculation and [Motion](overview.md) for service and hardware execution.

Relevant implementation:

- `crates/nodes/behavior_node/src/walk.rs`: behavior walking actions and path construction.
- `crates/booster/src/walking.rs`: path-to-velocity conversion.
- `crates/nodes/booster_sdk_interface/src/lib.rs`: movement RPC dispatch.

The previous development note about individual-step planning is preserved in [Historical: Step planning](../../historical/motion/step_planning.md).
