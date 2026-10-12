# Fall detection

The `fall_detection` node derives posture from measured IMU roll/pitch, angular velocity, and serial joint positions/velocities.
It publishes `fall_detection/status` with the sensor source time and a `valid_until` time `detection_lifetime` later.
Consumers reject future and expired messages, so a stopped detector cannot leave a stale posture behind.

## Postures

| Posture | Entry condition |
| --- | --- |
| `Upright` | Torso tilt below `upright.maximum_torso_tilt`, the torso at least `upright.minimum_torso_height_above_soles` above both soles, and limited torso and leg motion, continuously for `upright.confirmation_duration`. |
| `Falling` | From `Upright`, torso tilt exceeds `falling.minimum_torso_tilt` for `falling.confirmation_duration`. Also entered when a stand up reaches `standing_up.timeout` without confirmed standing. |
| `Fallen { ready_for_standup }` | Torso tilt exceeds `fallen.minimum_torso_tilt` with limited torso and joint motion, continuously for `fallen.confirmation_duration`. |
| `StandingUp` | From `Fallen { ready_for_standup: true }`, Behavior requests `StandUp`. Ends when standing is confirmed or at `standing_up.timeout`. |

The torso height check keeps a seated or kneeling robot with a vertical torso from counting as `Upright`.
Once `Upright`, only torso tilt is checked, so walking and kicking do not have to hold a standing pose.
Lying or rolling during `StandingUp` is part of the recovery and does not end it.

Confirmation durations must be observed without interruption.
A short gap in sensor input restarts them but keeps the confirmed posture.
After a gap longer than `detection_lifetime`, the detector starts over.

## Get-up readiness

`ready_for_standup` requires every non-head joint to stay within `standing_up.maximum_start_pose_joint_position_deviation` of `standing_up.start_pose` and slower than `standing_up.maximum_start_pose_joint_velocity` for `standing_up.confirmation_duration`.
It stays latched while Behavior requests `StandUp`, because the policy can move away from the accepted starting pose before the request reaches the detector.

This is a configured pose envelope.
It does not prove collision clearance or detect every crossed limb.

## Behavior

| Posture | Behavior action |
| --- | --- |
| Missing, stale, future, or `Falling` | `Damping` |
| `Fallen { ready_for_standup: false }` | `Prepare` |
| `Fallen { ready_for_standup: true }` or `StandingUp` | `StandUp` |
| `Upright` | Ordinary behavior |

Explicit primary states retain their existing priority.
Recovery runs after `Initial` and within remote control, so `Stop`, `Finished`, and `Penalized` do not initiate a new recovery.
