# Robot 43: stationary drift and slow estimation (2026-09-27)

This records the deployed configuration and earlier raw/averaged-IMU experiments.
For the current local implementation and matched 60-second calibration comparison,
see [100 ms preintegration](localization_preintegration.md).

## Evidence

The user confirmed the robot was stationary. The running executable on `10.1.24.43`
matches the local `target/container/aarch64-unknown-linux-gnu/debug/hulk_ros_z` exactly:
SHA-256 `fea5516569e7c7c32fb5a2910063146179baff81ba7c3649a1a5d85865e2097a`.
This repository's dev profile is optimized (`opt-level=3`, no debug assertions or overflow
checks). The deployed source snapshot selects LM + dense Cholesky, fagra `7b37df61`.

The deployed accelerometer configuration has zero bias, unit scale, zero lever arm,
noise density 0.3, and 10 ms averaging. Kinematic odometry is disabled. Bias is not a
graph state. Visual feature noise variance is 10000 (100 px standard deviation).

Downloaded a snapshot of the still-open recording:

- Remote: `/home/booster/hulk/logs/2026-09-27T17:24:08.443+08:00/recording.mcap`.
- Local: `/tmp/opencode/robot43-recording.mcap`.
- SHA-256: `32e0e3a8dd5221dd00e2b6ab88d4f7da293f71e426a1fc02f4298f3d389814f7`.
- It lacks a finalized footer; analysis used a linear reader through complete records.
- Decoded 961 solve diagnostics, 959 estimates, 34 status messages, 235854 low states,
  16253 VO messages, and 7766 visual association messages.

## Accelerometer inconsistency is sufficient to explain no-VO drift

During the first stationary minute, SDK orientation was nearly constant (approximately
5.89° roll, 13.54° pitch). Measured specific force minus the static prediction
`Rᵀ [0, 0, 9.81]` averaged approximately `[0.013, 0.028, -0.002]` m/s².
Gyro means were around `1e-4` rad/s. These data do not indicate a gross gravity sign or
unit error. They do show a persistent force discrepancy which the current model can
only explain by changing trajectory/tilt.

An offline ablation replayed roughly 29 seconds from the first recorded estimate through
the first 30 seconds of the recording. All variants used the same initial recorded Local
pose, raw IMU, reconstructed sole kinematics, camera calibration, and 50 ms delivery-time
solve schedule. VO was optionally included with interpolated camera extrinsics. Visual
landmarks were excluded to isolate local motion. No zero-velocity or contact-equality
factors were added.

A fixed effective bias was estimated from 2496 IMU samples in the first five seconds:

```
Robot-frame bias = [0.0115199745, 0.0281215876, 0.0014635906] m/s²
```

| Accelerometer treatment | VO | Maximum displacement from initial pose | Published estimates | Gradient-converged solves |
| --- | --- | ---: | ---: | ---: |
| Current zero-bias configuration | Off | **12.798836 m** | 564/565 | 560/565 |
| Subtract effective stationary bias | Off | **0.146820 m** | 564/565 | 556/565 |
| Current zero-bias configuration | On | **0.563766 m** | 564/565 | 558/565 |
| Subtract effective stationary bias | On | **0.008593 m** | 564/565 | 556/565 |
| Accelerometer disabled | On | **0.011370 m** | 564/565 | 493/565 |

This strongly supports the reported observation that disabling VO worsens divergence.
It also demonstrates that **numerical convergence does not imply correct motion**:
the uncalibrated no-VO graph converged in 560/565 solves while drifting almost 13 m.

This is an effective correction for one stationary orientation and temperature, not a
complete hardware calibration. Accelerometer bias, scale error, SDK tilt error, and
gravity-model error are not separately identifiable from this one pose. Temperature
dependence was not measured. Do not apply these values globally or treat them as a
validated multi-orientation calibration.

## Runtime has additional causes

1. **Live-system contention.** CPU clocks were at 1.984 GHz, MAXN_SUPER mode, approximately
   69°C. Four cores were typically 90–100% busy. The full system had load averages around
   12–14, and logs also contained head/motion RPC timeouts. No clock throttling was observed
   during the sample. The identical isolated tracking benchmark executable, run with the
   robot application still active, measured:

   | Target | Median cycle | p95 | Total for 6 s of data | Field estimates |
   | --- | ---: | ---: | ---: | ---: |
   | Idle robot 99, earlier confirmation | 36.345 ms | 55.478 ms | 4.257 s | 119/121 |
   | Loaded robot 43 | 104.246 ms | 165.130 ms | 12.275 s | 119/121 |

   This is about **2.9× slower with identical inputs and solver code**, independently of
   the recorded sensor bias. It is a cross-robot/load comparison, not an isolated causal
   CPU-contention experiment.

2. **Repeated recovery work.** After 120 seconds of the live recording, full
   `estimation_duration` was 1094.8 ms median / 1495.9 ms p95; the selected estimator's
   narrower `duration` was 442.8 ms median / 681.8 ms p95. Median ingestion was only
   2.39 ms. `Localization::solve` also constructs, replays, solves, and checks global
   recovery candidates while LostTrack. Their time is included in the outer metric, even
   when discarded. The recording repeatedly alternates brief Tracking with LostTrack
   and reaches generation 14. Expensive retries consume much of the two-second freshness
   budget; published estimate age in this later interval was 1107.8 ms median.

3. **Different graph workload.** The live node includes both sole nonpenetration factors
   at kinematics rate, whereas the earlier benchmark deliberately omitted feet. The live
   graph had roughly 2100–2500 counted observations and at most 16 states after retirement;
   there is no evidence here of unbounded state-count growth. Late solves commonly use
   8–10 accepted iterations, compared with 2–4 early on.

Overall, 813/961 selected solves reported GradientTolerance, 137 MaxIterations, 11
NoProgress, and only two explicit failures (initial singular information). This does not
count convergence/failure of discarded bootstrap candidates: the current diagnostics
publish the selected attempt's numerical statistics, plus an outer total duration.

## Recommended follow-up

- Calibrate accelerometer bias/scale and Robot-frame mounting geometry; use stationary
  multi-orientation measurements to separate sensor errors from attitude errors.
- Add observable, regularized bias estimation if temperature/time drift must be tracked;
  account for SDK-attitude-dependent acceleration averaging and replay/marginalization.
  Do not learn bias from presumed stationary feet during unknown contact or flight.
- Address the recovery retry/latency problem separately and measure under the complete
  robot workload. Instrument candidate outcomes and stage timings before tuning solver
  stopping criteria. Merely loosening convergence thresholds cannot correct inertial drift.

No live parameters, services, or deployed executables were changed. The separate benchmark
test executable was copied to `/tmp/localization-benchmark-upstream`. Diagnostic utilities,
decoded JSONL, and the ablation source are retained locally under `/tmp/opencode/robot43-*`;
the compiled host ablation can be rerun as:

```sh
target/debug/examples/replay_stationary /tmp/opencode/robot43-recording.mcap
```

## Online calibration validation

The implementation now includes independent, linearly interpolated 5-second bias knots,
joint marginalization, calibrated averaging/lever-arm Jacobians, and bias covariance diagnostics.
The earlier investigation above describes the original fixed-calibration implementation.

Tested the new release executable **on robot 43**, with the full robot application still running,
against the first 30 seconds of the same confirmed-stationary recording. Initialization uses the
first recorded Local pose; the roughly 29-second replay supplies actual IMU, sole kinematics,
and optionally VO. It schedules solves every 50 ms of delivery time. It does not supply global
landmarks, stationary constraints, or a measured calibration value to the graph. The stationary
force residual from the first five seconds is printed solely as an independent reference.

The final learned accelerometer bias with VO was:

```
Graph estimate:       [0.011807, 0.028303, 0.001680] m/s²
Stationary reference: [0.011520, 0.028122, 0.001464] m/s²
```

| Mode | Final displacement | Maximum displacement, entire run | Estimates | Gradient-converged solves | Median cycle | p95 cycle |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Bias constrained near zero, VO on | 0.597 m | 0.609 m | 564/565 | 445/565 | 232.8 ms | 341.7 ms |
| Online bias, VO on | **0.030 m** | 0.430 m | 564/565 | 565/565 | 153.4 ms | 248.4 ms |
| Online bias, VO disabled after 20 s | **0.008 m** | 0.430 m | 564/565 | 565/565 | 166.2 ms | 306.5 ms |
| Online bias, no VO from startup | 7.326 m | 7.326 m | 564/565 | 565/565 | 155.6 ms | 245.0 ms |

All runs have one initial covariance failure; a gradient-converged solve can still fail covariance.
The fixed-zero ablation uses the same augmented graph with very tight zero-bias priors/walks,
not the old executable, so its conditioning and timing differ from the old benchmark. These are
single runs under varying live-system load, not a controlled speed comparison. Total processing
was 130.91, 87.61, 97.38, and 87.36 seconds respectively, excluding recording I/O and decoding.

A separate back-to-back check of the original 6-second synthetic tracking workload on the loaded
robot took 9.95 s with the pre-calibration executable (80.0 ms median, 154.7 ms p95), and 20.03 s
with online bias (146.2 ms median, 371.2 ms p95). Both published 119/121 field estimates; maximum
errors changed from 1.98 mm / 0.069° to 9.63 mm / 0.241°. The extra calibration states and their
coupling have a material runtime cost despite the coarse grid. This is not a real-time-performance
fix; further profiling/solver conditioning work is still needed under the live robot workload.
Subsequent unchanged-objective optimizations and matched before/after measurements are recorded
in the [perf improvement report](localization_perf.md#implemented-improvements-and-matched-measurements).

The graph learns the observed offset and materially reduces stationary drift. There remains a
**learning transient**: initial displacement reaches 43 cm before VO identifies the offset;
maximum displacement after ten seconds is 19 cm. Calibration is not immediately known at startup.
In the no-VO run, estimated accelerometer sigma remains approximately 0.507 m/s² per axis,
versus roughly 0.062–0.069 with VO. Adding variables does not make IMU-only bias observable.
The lower endpoint error during the short VO outage is not evidence that removing VO helps:
it is one short continuation after learning on the same stationary record.

A separate 60-second host replay with continuous VO remained stable through repeated bias-knot
retirement, finishing at `[0.012170, 0.028201, 0.001670]` m/s² estimated accelerometer bias and
approximately 2.4 cm displacement. Temperature changes and general multi-orientation hardware
calibration have not been validated by this stationary experiment.

Validation also passed:

- 26 localization tests on robot 43, including biased-motion recovery, marginalization, rollback,
  averaged bias interpolation across a knot boundary with nonzero lever arm, and double flight.
- 144 factor/variable tests on the host, including independent AD Jacobian checks in f32/f64.
- 42 simulator tests, two allocation checks, and the factor-schema doctest.
- The older divergence recording: 330/333 local estimates, no field estimates, max |height| 0.605 m.

Reproduction:

```sh
target/debug/pepsi build crates/nodes/localization-3d --env podman --release --example imu_calibration_replay --tests
# Copy release/examples/imu_calibration_replay to the robot, then run:
/tmp/imu_calibration_replay /home/booster/hulk/logs/2026-09-27T17:24:08.443+08:00/recording.mcap 30
# Optional third argument selects fixed-zero, learn-with-vo, learn-vo-outage, or learn-without-vo.
```

Robot executable SHA-256: `7021062c3334e72421c377ec050130ca8e2b42f926d17b71f29ed931fd249093`.
Raw results: `/tmp/opencode/robot43-calibration-robot.txt` on the host. This was an isolated
recorded-data replay on the robot, not a deployment of the updated controller service.
