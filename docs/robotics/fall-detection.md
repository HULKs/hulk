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
| `StandingUp` | From `Fallen { ready_for_standup: true }`, Behavior requests `StandUp`. Ends when standing is confirmed, when the robot settles into `Fallen` after confirmed recovery movement, or at `standing_up.timeout`. |

The torso height check keeps a seated or kneeling robot with a vertical torso from counting as `Upright`.
Once `Upright`, only torso tilt is checked, so walking and kicking do not have to hold a standing pose.

A standup starts in a settled lying pose, so the `Fallen` guards initially cannot end the attempt.
The detector first requires the fallen conditions to be false continuously for `standing_up.movement_confirmation_duration`, 50 ms by default.
This means torso tilt drops below `fallen.minimum_torso_tilt`, torso angular speed reaches `fallen.maximum_torso_angular_velocity`, or the absolute speed of any non-head joint reaches `fallen.maximum_joint_velocity`.
Once confirmed, this departure enables a return to `Fallen` for the rest of the attempt.
If all the fallen conditions subsequently hold for `fallen.confirmation_duration`, the detector publishes `Fallen { ready_for_standup: false }`, requiring fresh preparation and readiness before retrying.
Each new attempt must confirm its own departure.
The 4-second timeout remains a fallback to `Falling` when standing conditions are not met, including when recovery movement never starts.

Confirmation durations must be observed without interruption.
A short gap in sensor input restarts them but keeps the confirmed posture.
It also preserves already confirmed recovery movement within the current attempt.
After a gap longer than `detection_lifetime`, the detector starts over.

## Get-up readiness

`ready_for_standup` requires all of these conditions to hold together while `Fallen`, for `standing_up.confirmation_duration`:

- The torso is within `standing_up.maximum_start_pose_torso_deviation` of lying flat on its front or back. Side-lying and upright poses do not qualify.
- Torso angular speed is below `standing_up.maximum_start_pose_torso_angular_velocity`.
- Every non-head joint is within `standing_up.maximum_start_pose_joint_position_deviation` of `standing_up.start_pose` and slower than `standing_up.maximum_start_pose_joint_velocity`.

The default torso limits are 30 degrees and 0.35 rad/s, held with the joint conditions for 300 ms.
Leaving this envelope before a standup request clears readiness and restarts confirmation.
It stays latched while Behavior requests `StandUp`, because the policy can move away from the accepted starting pose before the request reaches the detector.

This is a configured pose envelope.
It does not prove ground contact or collision clearance, or detect every crossed limb.

## Behavior

| Posture | Behavior action |
| --- | --- |
| Missing, stale, future, or `Falling` | `Damping` |
| `Fallen { ready_for_standup: false }` | `Prepare` |
| `Fallen { ready_for_standup: true }` or `StandingUp` | `StandUp` |
| `Upright` | Ordinary behavior |

Explicit primary states retain their existing priority.
Recovery runs after `Initial` and within remote control, so `Stop`, `Finished`, and `Penalized` do not initiate a new recovery.
