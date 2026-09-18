# Motion safety and recovery handover

The shared `fall_detection` node supplies the physical posture used by Behavior,
Motion, odometry, support-foot estimation, obstacle filtering, and team reporting.
The Booster `FallDownState` receiver remains only for raw diagnostic recording.
It no longer authorizes walking or determines recovery completion.

## Control contract

- Motion requires fresh behavior commands (100 ms), body sensors (50 ms), the
  shared fall estimate (50 ms), and hardware status (100 ms). The detector also
  invalidates its estimate after 50 ms without a usable sensor sample.
- Ordinary Walk/Kick output requires Upright. This is checked before inference
  and again before dispatch, including injected and remote commands.
- Falling requires 45 degrees of torso tilt for 20 ms. Fallen requires at least
  60 degrees of tilt and gyro norm below 2.5 rad/s for 150 ms. These thresholds
  are conservative intervention rules, not a validated predictor of an imminent
  fall. Transitions between Fallen and Falling are expected during recovery.
- A fresh StandUp request in Fallen authorizes one Motion-owned recovery attempt.
  Repeated requests do not reset its progress. Expected recovery tilts are allowed
  only during that attempt. Explicit Damping and Prepare cancel it.
- Slow get-up completion requires actual inference progress reaching 1 and a
  measured walking posture. Progress starts with actual policy execution, after
  the Custom-mode acknowledgement. With the current model parameters, progress
  reaches 1 after 3.5 s for front recovery or 4 s for back recovery.
- The measured posture requires torso tilt below 20 degrees, gyro norm below
  4 rad/s, leg joint velocity RMS below 5 rad/s, maximum absolute hip pitch below
  1.2 rad, hip roll below 0.5 rad, and knee flexion below 1.5 rad, sustained for
  100 ms. Upright alone does not complete recovery.
- Fast get-up has no progress input. It uses the same measured posture after a
  minimum of 1 s of policy execution. This rule has not been validated against
  a captured fast recovery.
- Recovery has an 8 s overall deadline. Slow get-up may spend at most 500 ms at
  its endpoint without an acceptable walking posture. Expiry latches a fault.
- Completion selects the Walk policy at zero velocity on the next control cycle.
  Kick and nonzero walking requests remain inhibited during settling. Settling
  requires at least 200 ms of accepted Walk execution, measured readiness, and a
  newer behavior request other than StandUp; it has a 2 s deadline from Walk
  execution. Hardware acknowledgement and inference deadlines bound activation
  before the first Walk output. The walking policy continues without a reset
  when ordinary behavior resumes.
- Data loss, inference/head-service failure, and hardware faults latch damping.
  Fresh data alone cannot restart motion. Rearming requires Damping followed by
  Prepare, with valid inputs. A protective Damping request does not hide sensor
  loss. A physical fall itself inhibits ordinary motion and permits a subsequent
  explicitly requested recovery.

Readiness constrains activation and recovery completion; ordinary walking does
not have to remain within the recovery joint-pose envelope on every step.

## General safety fixes

| Finding | Implemented behavior |
|---|---|
| Stale/future/nonfinite sensor data | Validate at receipt, inference dispatch/completion, and Motion dispatch; faults inhibit output. Expired/out-of-order backlog cannot renew a lease. |
| Queued or late inference | One pending request, generation and deadline checks, rejection of superseded/late results, and policy/history reset after interruption. |
| Body continues after head failure | Any failed head or body request faults the whole Motion output. |
| Unsafe output or parameter values | Validate joint-limit messages on receipt, finite outputs and nonnegative gains, positive get-up progression, and finite walking geometry. Re-clamp against current limits at Motion and actuator boundaries. |
| Unacknowledged mode changes | Protective joint commands until acknowledgement of the current mode-request generation; older acknowledgements cannot authorize a new transition. |
| Last target retained after producer loss | Independent actuator command watchdog (100 ms), streamed protective damping targets, and a bounded mode transition (2 s). |
| Hidden worker failures | Propagate worker errors; own/abort the mode task; attempt protective output when the actuator loop fails. |
| Runtime configuration/model mismatch | Inference configuration and actuator output-period changes require restart. |

The joint clamp bounds positions and rejects nonfinite values and negative gains.
It is not a bound on the actuator's internal PD torque. `maximum_torque` remains
URDF/configuration data; firmware enforcement has not been established.

## Recording

In addition to low-state, kinematics, commands, behavior, and SDK fall diagnostics,
MCAP records:

- `fall_detection/status`: physical posture, readiness, sample time, tilt, gyro.
- `motion/execution`: phase, generation, actual recovery start/progress, fault.
- `motion_inference/timing`: request/receipt/start/completion and selected sensor
  times, compute duration, acceptance or error.
- `hardware_interface/status`: desired mode, current acknowledged mode, command
  source time, and watchdog/mode fault.
- `hardware_interface/joint_command`: the exact LowCommand selected for
  publication, including protective targets. Publication is not proof of receipt
  or execution by the firmware.

## Validation on 2026-09-18

The production detector was replayed over **1,148,761 low-state frames** from all
five recordings in `~/logs/motion-test/10.1.24.42`. Each recording has an incomplete
tail; only intact records were read. The latest playing interval contains three
annotated falls. The initial Falling transitions preceded the old SDK flag by
approximately 1.16 s, 0.65 s, and 1.04 s. Other Falling transitions in that interval
were inside those same fall/recovery windows. This corpus does not establish a
zero false-positive rate across other motions, handling, or environments.

The production recovery state machine was replayed against the three measured
front/back/front recovery windows, retaining the recorded request starts:

| Recovery | Zero-velocity Walk selected (robot-local UTC+8) | Earlier than old exit | Settling completed |
|---|---|---|---|
| Front 1 | 02:32:26.617 | 1.467 s | 02:32:26.828 |
| Back | 02:32:45.138 | 1.965 s | 02:32:45.419 |
| Front 2 | 02:33:45.729 | 0.493 s | 02:33:45.944 |

These are counterfactual controller decisions using the recorded physical motion.
The recordings do not show the physical response to switching policies earlier.
Policy start times were not recorded then, so this replay anchors progress to the
behavior request and configured policy duration. The new diagnostics remove that
ambiguity from future recordings.

Actual ONNX inference and decoding passed finite-output checks on **107,794**
samples across all five models and recordings. It rejected **552** expired frames
and reset across **79** accepted-source gaps. Host compute time was approximately
0.0072 ms median, 0.0144 ms p99, and 0.505 ms maximum. These are development-host
CPU timings, not robot throughput or end-to-end latency. Cycling all models over
captured observations checks numerical behavior, not whether a policy is suitable
for every physical pose.

A temporary isolated ROS-Z harness ran the actual Motion, FallDetection,
Inference, HeadMotion, and HardwareInterface nodes with real ONNX models, a
recorded upright joint pose, controlled IMU/data interruptions, and simulated SDK
RPC acknowledgements. It verified initial mode handshake and settling, fallen
Walk suppression, recovery authorization, Prepare cancellation, latched sensor
loss even during Damping, deliberate rearming, and the independent actuator
watchdog after Motion stopped. A separate temporary check verified rejection of
an old Custom acknowledgement after an intervening mode request. Temporary test
sources were removed from the repository.

The latest playing interval also contains **eight delivery gaps over 50 ms**
(approximately 53–140 ms). No sample in that interval was itself over 50 ms old on
arrival. Depending on timer scheduling, those gaps can trigger the new safety
latch. Recorder loss and input-delivery loss cannot be distinguished from this
recording alone. Investigate these gaps before deployment; the freshness limits
were not relaxed to make the replay pass.

No closed-loop MuJoCo or live robot recovery was performed. The sibling simulator
requires a display and does not emulate SDK mode acknowledgements as shipped;
the isolated node harness covers software coordination without claiming physical
validation. Remaining physical checks include supported upright poses, fast and
side recoveries, handover stability, protective damping gains, and firmware
behavior after complete host/process loss. Software watchdogs cannot establish a
firmware watchdog guarantee.

Build checks passed for `hulk_ros_z` and `bevyhavior_simulator`, including all
compile targets. The 61 existing tests in Motion, Inference, HardwareInterface,
HeadMotion, Odometry, and SupportFootEstimator passed. Clippy passed for the four
motion safety crates with the pre-existing `too_many_arguments` lint in kick
observation construction exempted; formatting and diff checks passed.
