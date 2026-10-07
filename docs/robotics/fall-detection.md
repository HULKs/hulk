# Fall Detection

The `fall_detection` node publishes a `FallDetection` on `fall_detection/status` for every `inputs/low_state` sample.
It only uses measured data: the torso tilt from IMU roll and pitch, the gyro, and the joint positions and velocities.
Consumers treat a message older than `MAXIMUM_FALL_DETECTION_AGE` as unavailable.

## Postures

| Posture                         | Entered when                                                                                          |
| ------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `Upright`                       | tilt below `upright_tilt` for `upright_duration`                                                      |
| `Falling`                       | from `Upright`, tilt above `falling_tilt` for `falling_duration`                                      |
| `Fallen { ready_for_standup }`  | tilt above `fallen_tilt` with angular speed below `maximum_fallen_angular_speed` for `fallen_duration` |
| `StandingUp`                    | from `Fallen { ready_for_standup: true }` while behavior commands `StandUp`                           |

`StandingUp` ends as `Upright` once the robot is upright, or falls back to `Fallen` after `stand_up_timeout`.

`ready_for_standup` is set while every non-head joint is within `maximum_stand_up_pose_deviation` of `stand_up_pose` and slower than `maximum_stand_up_joint_speed` for `stand_up_stable_duration`.
It stays latched while behavior commands `StandUp`, because the stand up policy moves the joints out of that pose before the request reaches the detector.

## Behavior

| Posture                                       | Motion      |
| --------------------------------------------- | ----------- |
| `Falling`                                     | `Damping`   |
| `Fallen { ready_for_standup: false }`         | `Prepare`   |
| `Fallen { ready_for_standup: true }`, `StandingUp` | `StandUp` |

`Prepare` stiffens the robot into the SDK prepare pose, which is the known, untwisted starting pose `stand_up_pose` describes.
Damping while falling runs right after the `Damping` and `Prepare` primary states, ahead of every other motion.
Stand up and `Prepare` run after the `Initial` primary state and within remote control, so no new stand up attempt is made during `Stop`, `Finished` or `Penalized`.
