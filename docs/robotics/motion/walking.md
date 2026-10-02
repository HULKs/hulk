# Walking

The walking path converts behavior's motion requests into
velocity commands for the Booster SDK `move_robot` RPC in Soccer mode.

## From Motion Command to Velocity

`crates/nodes/booster_sdk_interface/src/lib.rs` handles three walking-related commands:

- `Stand`: send zero linear and angular velocity.
- `WalkWithVelocity`: forward the requested Ground-frame linear velocity and angular velocity.
- `Walk`: convert a path, target orientation, orientation mode, alignment distance, and speed into velocity with `booster::walking::step_from_motion_command`.

Despite its name and `Step` return type, this function supplies forward/left
**velocity** and angular velocity to the SDK; it does not plan a discrete footstep.
Linear velocity is in m/s and angular velocity in rad/s.

## Path Following

`crates/booster/src/walking.rs` computes:

1. The forward direction of the path at the Ground origin.
2. Linear velocity along that direction, scaled by requested speed and a deceleration factor `clamp(path_length / deceleration_distance, 0, 1)`.
3. A walking orientation derived from the path, a requested look direction, or a requested look-at point.
4. A blend from walking orientation to target orientation near the destination. Alignment importance is one inside `distance_to_be_aligned`, zero beyond that distance plus `hybrid_align_distance`, and cosine-interpolated between them.
5. Angular velocity from the blended orientation's sine multiplied by `max_alignment_rate`.

This conversion has no explicit empty-path, nonfinite-geometry, or
coefficient-validation gate; supply valid paths and parameters.

## SDK Execution

The SDK interface sends `move_robot` requests when its locally assumed mode is
Soccer and the movement interval has elapsed. Head targets arrive independently
on `head_joints_command` and are sent via `rotate_head`. Main does not execute
a repository-owned walking inference service or publish Custom joint commands.

Path-following parameters are under `booster_interface.walking` in
`etc/parameters/base/booster_interface.json5`; the same file configures the
movement and head RPC intervals and SDK request timeout.

See [Step planning and path following](step_planning.md) for how paths relate to this interface.
