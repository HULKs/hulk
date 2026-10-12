# Fall detection

The `fall_detection` node derives posture from measured IMU roll/pitch, angular velocity, and serial joint positions/velocities.
It publishes `fall_detection/status` with the sensor source time.
Consumers reject future messages and messages older than `MAXIMUM_FALL_DETECTION_AGE`.

## Postures

| Posture | Entry condition |
| --- | --- |
| `Upright` | Torso tilt below `upright_tilt`, both soles at least `minimum_upright_foot_depth` below the torso, and limited torso and leg motion, continuously for `upright_duration`. |
| `Falling` | From `Upright`, torso tilt exceeds `falling_tilt` for `falling_duration`. Also entered when a stand up reaches `stand_up_timeout` without confirmed standing. |
| `Fallen { ready_for_standup }` | Torso tilt exceeds `fallen_tilt` with limited torso and joint motion, continuously for `fallen_duration`. |
| `StandingUp` | From `Fallen { ready_for_standup: true }`, Behavior requests `StandUp`. Ends when standing is confirmed or at `stand_up_timeout`. |

The foot depth check keeps a seated or kneeling robot with a vertical torso from counting as `Upright`.
Once `Upright`, only torso tilt is checked, so walking and kicking do not have to hold a standing pose.
Lying or rolling during `StandingUp` is part of the recovery and does not end it.

Confirmation durations must be observed without interruption.
A short gap in sensor input restarts them but keeps the confirmed posture.
After a gap longer than `MAXIMUM_FALL_DETECTION_AGE`, the detector starts over.

## Get-up readiness

`ready_for_standup` requires every non-head joint to stay within `maximum_stand_up_pose_deviation` of `stand_up_pose` and slower than `maximum_stand_up_joint_speed` for `stand_up_stable_duration`.
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
