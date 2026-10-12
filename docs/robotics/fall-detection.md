# Fall detection

The `fall_detection` node derives posture from measured IMU roll/pitch, angular velocity, and serial joint positions/velocities. It publishes `fall_detection/status` with the sensor source time. Invalid, future, or excessively old inputs reset confirmation history and do not produce a status. Consumers reject future messages and messages older than `MAXIMUM_FALL_DETECTION_AGE`, currently 100 ms.

## Postures and recovery exits

| Posture | Entry condition |
| --- | --- |
| `Upright` | Torso attitude, both leg poses, foot depth, and motion satisfy the standing checks continuously for `upright_duration`. |
| `Falling` | From `Upright`, torso tilt exceeds `falling_tilt` for `falling_duration`. |
| `Fallen { ready_for_standup }` | Torso tilt exceeds `fallen_tilt`, with limited angular velocity and non-head joint velocities, continuously for `fallen_duration`. An active recovery must first reach its timeout. |
| `StandingUp` | From `Fallen { ready_for_standup: true }`, Behavior requests `StandUp`. |
| `RecoveryFailed` | Recovery reaches its timeout without confirmed standing or sustained, settled lying. |

Every entry to `Upright`, including initialization and recovery from `Falling` or `RecoveryFailed`, requires all of these conditions for 100 ms with the current configuration:

- Torso tilt below 20 degrees and angular speed below 2.5 rad/s.
- Both legs within the configured per-joint envelope around the Prepare reference. Hip pitch, roll, yaw, knee, and ankle deviations are checked independently.
- Both sole centers more than 0.43 m below the torso origin, using measured joint angles and forward kinematics.
- Every leg joint slower than 6 rad/s.

The envelope admits the crouched stance measured at the end of successful get-ups. It rejects deep sitting, kneeling, and a folded leg even when the torso is vertical. The velocity ceilings allow the terminal oscillation seen in the recordings; they do not imply that the robot is motionless. Arms and head do not participate in standing confirmation. Once `Upright`, the existing torso-tilt hysteresis remains in effect so ordinary walking does not have to hold the entry pose.

During `StandingUp`, lying at the start or while rolling is part of the recovery. The detector therefore waits for the existing 8 s recovery deadline before declaring failure. At that deadline:

- Confirmed standing takes precedence and produces `Upright`.
- Tilt of at least 60 degrees, angular speed below 2.5 rad/s, and all non-head joint speeds below 1 rad/s for 150 ms produce `Fallen`.
- Any other configuration produces `RecoveryFailed`, which requests Damping and forbids learned motion. A seated or moving robot is not marked `Fallen` just because time elapsed.

`RecoveryFailed` can leave for `Upright` or `Fallen` after the corresponding physical confirmation. It does not immediately retry a get-up from an ambiguous pose.

## Get-up readiness

`ready_for_standup` requires every non-head joint to stay within `maximum_stand_up_pose_deviation` of `stand_up_pose` and slower than `maximum_stand_up_joint_speed` for `stand_up_stable_duration`. It stays latched while Behavior requests `StandUp`, because the policy can move away from the accepted starting pose before the request reaches the detector.

This is a configured pose envelope. There are no supplied trained starting-pose limits, and the envelope does not prove collision clearance or detect every possible crossed limb.

## Behavior and Motion

| Posture | Behavior action |
| --- | --- |
| Missing, stale, future, `Falling`, or `RecoveryFailed` | `Damping` |
| `Fallen { ready_for_standup: false }` | `Prepare` |
| `Fallen { ready_for_standup: true }` or `StandingUp` | `StandUp` |
| `Upright` | Ordinary behavior |

Explicit primary states retain their existing priority. Recovery runs after `Initial` and within remote control, so `Stop`, `Finished`, and `Penalized` do not initiate a new recovery.

Motion also checks posture before inference and before dispatching its result. Stand, Walk, and Kick require fresh `Upright`; GetUp requires fresh `StandingUp` or ready `Fallen`. Missing, stale, future, or incompatible status produces Damping. This also covers injected commands and primary-state overrides. Explicit Damping and Prepare remain available.

After inference, Motion rereads the latest stamped command and posture before sampling the clock. An expired or future command produces Damping. A new Damping or Prepare request takes effect instead of the pending inference result. Otherwise, both the inferred command and latest requested command must still be permitted by the current posture.

## Recorded validation and limits

The transition implementation was replayed against the three recoveries in `10.1.24.42/2026-09-18T02:25:46.212+08:00/recording.mcap`. The recording has an incomplete tail; 455,347 intact sensor frames were extracted. Each recovery replay assumes an accepted get-up at the recorded start and feeds the captured sensor samples through the production observation conversion and detector.

| Recovery | Torso-only confirmation | New confirmation | Recorded Behavior exit |
| --- | ---: | ---: | ---: |
| Front 1 | 3.209 s | 3.849 s | 5.002 s |
| Back | 3.988 s | 3.988 s | 5.996 s |
| Front 2 | 3.188 s | 3.456 s | 4.017 s |

Times are relative to the recorded get-up request. The front recoveries fail the new leg checks at their earlier torso-only confirmation. Temporary checks also covered seated/kneeling/folded-leg poses, transient upright poses, excessive joint/torso motion, timeout classification, invalid samples, stale status, and cancellation during inference.

These recordings helped set the envelope; they are not independent validation of the thresholds. The replay does not establish that earlier policy handover succeeds physically, nor does it validate get-up starting-pose authorization. IMU and joint kinematics cannot prove foot contact or balance when a robot is held upright. Controlled robot testing is still needed, including seated and kneeling poses and unsuccessful recoveries.
