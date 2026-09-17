# Kick and get-up requests

Behavior publishes `types::motion_command::MotionCommand` on
`behavior/motion_command`. Its `Kick` and `StandUp` variants carry the
policy choices through the motion node to inference. The motion node's output
`MotionCommand` contains the desired control mode and joint commands.

## Behavior inputs

`BodyMotion::Kick` and `MotionCommand::Kick` carry:

- `ball_position`: observed ball position in Ground coordinates, in metres.
- `ball_velocity`: current ball velocity in Ground coordinates, in m/s.
- `kick_direction`: desired kick orientation in Ground coordinates.
- `target_speed`: desired outgoing ball speed in m/s.
- `soft`: select the soft-kick policy.
- `quick`: enable the normal kick policy's quick flag.
- `strong`: enable the normal kick policy's strong flag, requesting maximum speed.

The existing `target_position` and `robot_theta_to_field` fields remain available
to behavior and tooling; motion does not pass them to inference.

The common behavior `kick` action prefers the visual percept for position and
uses `world_state.ball.ball_in_ground_velocity` for velocity. The visual selector
currently publishes zero velocity, so it is not a velocity source. Without a
current tracked ball, the request uses zero velocity rather than a cached
last-ball velocity. Position and velocity are assumed to describe the same ball.
Interception computes a future point locally while retaining the observed
position and current velocity in the request.

Initial choices come from `behavior_node.kicking.soft`, `.quick`, and
`.target_speed` (defaults: `false`, `false`, and `3.4`). A continuing kick retains
these choices and its strong flag while refreshing position, velocity, and aim.
Goalkeeper and penalty actions can still override the strong flag explicitly.
The walk fallback preserves the same settings while a transition is blocked.

`StandUp { fast }` selects the fast or slow get-up policy. The behavior action
reads `behavior_node.stand_up.fast` (default `false`) on entry and retains that
choice until a different command is produced, preventing parameter updates from
restarting recovery with a different policy.

## Inference and simulation

Motion forwards the fields without applying policy limits. Inference caps ball
velocity and clamps target speed to the selected policy's limits. Strong normal
kicks use the maximum speed regardless of `target_speed`. Soft kicks ignore the
strong and quick flags. Base speed limits are `[0.5, 3.4]` m/s for normal kicks and
`[0.1, 1.7]` m/s for soft kicks.

The behavior simulator approximates these speed rules with configurable limits.
It does not model the learned quick-kick movement or fast/slow recovery timing;
get-up still clears the simulated fallen state immediately.

## Serialized commands

Injected commands use the `Kick` variant and must provide the new kick fields
explicitly. Stand-up commands
use an object, for example `{"StandUp":{"fast":true}}`, instead of the old unit
variant string `"StandUp"`. Old serialized commands and recordings require
migration; no legacy deserialization fallback is provided. Message schemas and
consumers must be updated together.
