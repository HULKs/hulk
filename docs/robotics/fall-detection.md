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
| `StandingUp` | From `Fallen { ready_for_standup: true }`, Behavior requests `StandUp`. Ends when standing is confirmed, when the robot settles into `Fallen` after confirmed movement, or at `standing_up.timeout`. |

The torso height check keeps a seated or kneeling robot with a vertical torso from counting as `Upright`.
Once `Upright`, only torso tilt is checked, so walking and kicking do not have to hold a standing pose.

A standup starts in a settled lying pose, so the `Fallen` guards initially cannot end the attempt.
The detector first requires at least one fallen condition to be violated continuously for `standing_up.movement_confirmation_duration`, 50 ms by default.
This can be torso tilt, torso angular speed, one joint's speed, or the sum of absolute body joint speeds.
Once confirmed, movement enables a return to `Fallen` for the rest of the attempt.
If all the fallen conditions subsequently hold for `fallen.confirmation_duration`, the detector publishes `Fallen { ready_for_standup: false }`, requiring fresh preparation and readiness before retrying.
Each new attempt must confirm its own movement.
This is a sustained-stillness heuristic: a recovery that deliberately pauses while lying down for the entire confirmation duration can still be interrupted.
It does not require a particular torso trajectory and cannot identify a failed policy that continues moving.
The 4-second timeout bounds an attempt even when standing confirmation is pending or the robot never leaves the initial pose.

Confirmation durations must be observed without interruption.
A short gap in sensor input restarts them but keeps the confirmed posture.
It also preserves already confirmed movement within the current attempt.
After a gap longer than `detection_lifetime`, the detector starts over.

## Get-up readiness

`ready_for_standup` requires all of these conditions to hold together while `Fallen`, for `standing_up.confirmation_duration`:

- The torso is within `standing_up.maximum_start_pose_torso_deviation` of lying flat on its front or back. Side-lying and upright poses do not qualify.
- Torso angular speed is below `standing_up.maximum_start_pose_torso_angular_velocity`.
- Every non-head joint is within `standing_up.maximum_start_pose_joint_position_deviation` of `standing_up.start_pose` and slower than `standing_up.maximum_start_pose_joint_velocity`.
- The sum of absolute non-head joint velocities is below `standing_up.maximum_start_pose_joint_velocity_sum`.

The default torso limits are 20 degrees and 0.25 rad/s, held with the joint conditions for 200 ms.
Leaving this envelope before a standup request clears readiness and restarts confirmation.
It stays latched while Behavior requests `StandUp`, because the policy can move away from the accepted starting pose before the request reaches the detector.

This is a configured pose envelope.
It does not prove ground contact or collision clearance, or detect every crossed limb.

## Default thresholds

All velocity sums use absolute values before adding, so opposite joint movements cannot cancel.
The head is excluded.
The individual joint limit remains in force: small total movement must not hide one fast limb.
Readiness motion limits may not exceed the corresponding fallen limits.

| Transition | Default guard | Continuous confirmation |
| --- | --- | --- |
| `Upright` → `Falling` | Torso tilt ≥40° | 20 ms |
| `Falling` → `Fallen`, or armed `StandingUp` → `Fallen` | Tilt ≥60°, torso speed <0.5 rad/s, each body joint <1 rad/s, body speed sum <3 rad/s | 500 ms |
| `Fallen` → ready | Within 20° of front/back lying, torso speed <0.25 rad/s, each body joint within 0.5 rad of Prepare and <1 rad/s, body speed sum <2.5 rad/s | 200 ms |
| Ready `Fallen` → `StandingUp` | Fresh `StandUp` request | Immediate |
| `StandingUp` → enable fallen guard | At least one fallen condition violated | 50 ms |
| Unconfirmed, `Falling`, `Fallen`, or `StandingUp` → `Upright` | Tilt <20°, torso speed <2.5 rad/s, each leg joint <6 rad/s, torso >0.43 m above both soles | 100 ms |
| `StandingUp` → `Falling` | Attempt reaches 4 seconds without confirmed standing | Immediate |

The 40° falling trigger leaves 20° of hysteresis from upright confirmation and engages damping earlier than the former 45° trigger.
Fallen confirmation is deliberately slower than fall detection: damping continues while impact or limb motion settles.
The 500 ms fallen and 200 ms readiness windows require about 700 ms of settled input before a new recovery can start.
Standing confirmation retains its measured recovery envelope because terminal recovery oscillations exceed the settled-body limits.
Sensor and command freshness remain 50 ms; a published posture expires after 100 ms.

The `upright`, `falling`, and `fallen` confirmation durations confirm entry into their named posture.
For example, `falling.confirmation_duration` delays `Upright` → `Falling`, not leaving `Falling`.
`standing_up.confirmation_duration` instead confirms readiness while still `Fallen`;
entering `StandingUp` on a request is immediate once readiness is true.
`standing_up.movement_confirmation_duration` arms the later return to `Fallen`.

## Joint velocity debug output

Plot these scalar topics alongside `fall_detection/status`:

- `debug/fall_detection/maximum_joint_speed`: the largest absolute body joint velocity.
  Compare against `fallen.maximum_joint_velocity` and `standing_up.maximum_start_pose_joint_velocity`.
- `debug/fall_detection/joint_speed_sum`: the sum of absolute body joint velocities.
  Compare against `fallen.maximum_joint_velocity_sum` and `standing_up.maximum_start_pose_joint_velocity_sum`.

Both measurements exclude the head and use rad/s, matching the detector's guards.
They and `debug/fall_detection/tilt` use the sensor source timestamp and publish only for finite, fresh observations.
Both velocity measurements must remain strictly below their respective limits for the applicable confirmation duration;
torso and pose conditions must also hold.
To retain these plots in MCAP recordings, add the debug topics to `mcap_recorder.topics`.

## Threshold validation

The simulator failure recording contained ready-pose joint speed sums up to 1.8 rad/s.
Its interrupted standup segments stayed below the new fallen motion limits for at most 128 ms at a time.
An uninterrupted simulator recovery remained within the fallen limits for about 300 ms during normal motion.
The 2.5 rad/s readiness limit and 500 ms fallen window leave room around those measurements rather than matching their extrema.
With these defaults, fresh simulator front/back recoveries reached `Upright` in one attempt, about 3.41/3.96 seconds after entering `StandingUp`.
Three earlier hardware recovery traces confirmed `Upright` at 3.85, 3.99, and 3.41 seconds.
The interrupted recording can only check the 42 observed attempt prefixes; it cannot show what those attempts would have done after the original controller stopped them.
These measurements cover the recorded scenarios, not every recovery policy or hardware condition. Fresh hardware validation is still needed.

## Behavior

| Posture | Behavior action |
| --- | --- |
| Missing, stale, future, or `Falling` | `Damping` |
| `Fallen { ready_for_standup: false }` | `Prepare` |
| `Fallen { ready_for_standup: true }` or `StandingUp` | `StandUp` |
| `Upright` | Ordinary behavior |

Explicit primary states retain their existing priority.
Recovery runs after `Initial` and within remote control, so `Stop`, `Finished`, and `Penalized` do not initiate a new recovery.
