# Path planning

The behavior action `walk_to` accepts a target pose in `Ground`, including a position and orientation.
It checks whether the robot is within the configured position and heading tolerances.
If the robot still needs to move, behavior plans a path and publishes it as part of `MotionCommand::Walk`.
[Walking action](https://github.com/HULKs/hulk/blob/main/crates/nodes/behavior_node/src/walk.rs) · [Motion command type](https://github.com/HULKs/hulk/blob/main/crates/types/src/motion_command.rs) · [Coordinate frames](../perception/localization.md#coordinate-frames)

## Planning inputs

The planner takes perceived obstacles, rule obstacles, field borders, goal structures, and the ball into account when those inputs are available.
The target passed to the planner is limited to the field and its configured border strip.
If planning returns no path, behavior falls back to a direct path to the requested target.
The result carries path geometry for the [walking controller](walking.md).
[Planning inputs and fallback](https://github.com/HULKs/hulk/blob/main/crates/nodes/behavior_node/src/walk.rs) · [Path planner](https://github.com/HULKs/hulk/blob/main/crates/path_planner/src/path_planner.rs) · [Obstacle tracking](../perception/filters.md#obstacles)
