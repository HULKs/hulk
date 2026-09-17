# Kick and get-up requests

Behavior publishes `types::motion_command::MotionCommand` on
`behavior/motion_command`. Its `Kick` and `StandUp` variants carry policy choices
through the motion node to inference. Motion publishes `RobotCommand` on
`commands/robot_command`, containing the desired control mode and joint commands.

## Behavior inputs

`BodyMotion::Kick` and `MotionCommand::Kick` carry:

- `ball_position`: observed ball position in Ground coordinates, in metres.
- `ball_velocity`: current ball velocity in Ground coordinates, in m/s.
- `kick_direction`: desired kick orientation in Ground coordinates.
- `target_speed`: desired outgoing ball speed in m/s.
- `soft`: select the soft-kick policy.
- `quick`: enable the normal kick policy's quick flag.
- `strong`: enable the normal kick policy's strong flag, requesting maximum speed.

`target_position` and `robot_theta_to_field` are available to behavior and
tooling; motion does not pass them to inference.

The behavior `kick` action prefers the visual percept for position and uses
`world_state.ball.ball_in_ground_velocity` for velocity. The visual selector
publishes zero velocity. Without a current tracked ball, the request uses zero
velocity. Position and velocity are assumed to describe the same ball.
Interception computes a future point locally while retaining the observed
position and current velocity in the request.

Initial choices come from `behavior_node.kicking.soft`, `.quick`, and
`.target_speed` (defaults: `false`, `false`, and `3.4`). Target speed is configured
independently of target distance. A continuing kick retains these choices and
its strong flag while refreshing position, velocity, and aim. Goalkeeper and
penalty actions can override the strong flag explicitly. The walk fallback
preserves the same settings while a transition is blocked.

`StandUp { fast }` selects the fast or slow get-up policy. The behavior action
reads `behavior_node.stand_up.fast` (default `false`) on entry and retains that
choice until a different command is produced.

## Behavior selection parameters

These parameters belong to `behavior_node.kicking`. All distances are in metres.

| Parameter | Base default | Purpose |
| --- | --- | --- |
| `allow_strong_kicks` | `false` | Permit strong striker and penalty kicks; this does not limit target speed. |
| `strong_kick_min_target_distance` | `6.0` | Select strong striker kicks at or beyond this ball-to-target distance. Penalty selection only checks permission. |
| `kick_activation_distance` | `2.0` | Centre of the normal striker's walk-to-kick handoff band. |
| `kick_activation_hysteresis` | `0.5` | Total width of that band: enter below 1.75, leave above 2.25. |
| `approach_ball_standoff` | `0.3` | Walking standoff behind the ball and set-play alignment position. |
| `minimum_interception_forward_distance` | `0.3` | Minimum Ground x coordinate of the predicted interception point, for both striker and goalkeeper. |

`disable_strong_kick` clears the strong flag without selecting the soft policy
or lowering `target_speed`. The `kick_strength_subtree` retains settings while
a kick continues. Goalkeeper actions disable strong explicitly.

Strength selection and kick direction use the same target position. The separate
`substates.distance_for_kick` and `substates.distance_for_kick_hysteresis`
parameters control proximity to the set-play alignment position.

## Inference and simulation

Motion forwards the fields without applying policy limits. Inference caps ball
velocity and clamps target speed to the selected policy's limits. Strong normal
kicks use the maximum speed regardless of `target_speed`. Soft kicks ignore the
strong and quick flags. Base speed limits are `[0.5, 3.4]` m/s for normal kicks and
`[0.1, 1.7]` m/s for soft kicks.

Inference caps the ball-position observation at 2.0 m. The base kick exit
distance is 2.25 m, so observations beyond the cap lose distance information.
The observation cap does not establish the policy's physical reach.

The behavior simulator approximates the speed rules with configurable limits.
It does not model the learned quick-kick movement or fast/slow recovery timing;
get-up clears the simulated fallen state immediately.

## Serialized commands

Injected kick commands use the `Kick` variant with all fields supplied explicitly.
Stand-up commands use `{"StandUp":{"fast":true}}` for fast recovery or
`{"StandUp":{"fast":false}}` for slow recovery.
