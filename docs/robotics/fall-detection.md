# Fall detection

`fall_detection` uses IMU data and joint positions and velocities to estimate the robot's posture.
It publishes the result on `fall_detection/status`, which behavior uses to protect the robot during a fall and coordinate getting up.

## Postures

| Posture | Meaning | Behavior response |
| --- | --- | --- |
| `Upright` | The robot has confirmed a standing posture. | Continue ordinary behavior. |
| `Falling` | The robot has tilted too far, or a get-up attempt has timed out. | Request `Damping`. |
| `Fallen` | The robot is lying still. | Prepare for getting up, then request `StandUp` once ready and allowed. |
| `StandingUp` | A get-up attempt is in progress. | Continue the attempt. |

Confirmation windows filter brief changes in posture and motion.
Missing or expired posture data makes behavior request `Damping`.
Explicit `Damping` and `Prepare` primary states take priority over fall recovery.

## Getting up

Before starting an attempt, the detector checks that the robot is lying steadily on its front or back, close to the configured starting pose.
Behavior only starts getting up when the current primary state permits it; otherwise, a fallen robot remains in damping.
A successful recovery returns to `Upright`.
If the robot settles back into a fallen posture after moving, it must prepare again before retrying.
An attempt that exceeds its configured timeout returns to `Falling` and damping.
[Get-up motion](motion/kicking.md#inference-and-simulation)

## Inspection and configuration

Inspect `fall_detection/status` in [Twix](../tooling/twix.md).
The topics under `debug/fall_detection/` expose torso tilt and joint speeds.
Thresholds, confirmation durations, and the recovery timeout are configurable.  
[Detector](https://github.com/HULKs/hulk/blob/main/crates/nodes/fall_detection/src/lib.rs) · [Parameters](https://github.com/HULKs/hulk/blob/main/etc/parameters/base/fall_detection.json5) · [Behavior integration](https://github.com/HULKs/hulk/blob/main/crates/nodes/behavior_node/src/tree.rs)
