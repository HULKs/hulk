# 100 ms IMU preintegration — implementation and robot43 comparison

## Decision

Retain preintegration: whole-process CPU time falls **80.6% on stationary replay and
80.1% on moving tracking** versus the preserved batched raw-IMU implementation, with
the same estimate availability and similar calibration/pose accuracy. This is a
measurement-model change, not a roundoff-equivalent batching optimization.

The 200 ms pose grid, two-second window, five-second linearly interpolated bias knots,
motion priors, SDK heading constraints, and fagra `7b37df61` LM/dense-Cholesky solver
remain in use. There is no GTSAM runtime dependency. Tests ran as isolated robot43
executables; the running robot application, affinity, and power settings were not changed.

## Model and covariance

Specific force is calibrated in Robot axes: `(raw - fixed_bias) * scale`. Integration
is at the **physical IMU origin**, avoiding numerical angular-acceleration differencing.
For gyro `w`, force `f`, interpolated bias `b`, and held sample duration `h`:

```text
a       = delta_R * (f - b_a)
delta_p = delta_p + delta_v*h + a*h²/2
delta_v = delta_v + a*h
delta_R = delta_R * Exp((w - b_g)*h)
```

Force uses the old rotation. Bias interpolation uses the step midpoint, with separate
sensitivities for both knots. Bias column order is gyro XYZ then accel XYZ. Actual
timestamps determine duration; aligned 100 ms boundaries also respect pose/bias boundaries.

Spline endpoint states are transformed to the sensor origin using `p_s = p + R*r` and
`v_s = v + R*(omega × r)`. With positive-up compensation `g = [0,0,9.81]`, residuals are:

```text
R: Log(corrected_delta_R⁻¹ * R0⁻¹ * R1)
V: R0⁻¹ * (v_s1 - v_s0 + g*dt) - corrected_delta_v
P: R0⁻¹ * (p_s1 - p_s0 - v_s0*dt + g*dt²/2) - corrected_delta_p
```

Endpoints are evaluated on the spline, not read from control values. The analytic
mounting-offset derivatives include both position and angular-velocity contributions.
Upright rest supplies +9.81 m/s²; **free fall supplies zero**, not a missing measurement.

The full 9×9 covariance uses `[right rotation, velocity in interval-start axes,
position in interval-start axes]`. Let `D = Exp((w-b_g)*h)ᵀ`,
`E = -delta_R*[f-b_a]×*h`, and `H = E*h/2`:

```text
F = [D, 0, 0; E, I, 0; H, h*I, I]
P = F*P*Fᵀ + Q
Q_RR = Jr((w-b_g)*h) * Jr((w-b_g)*h)ᵀ * gyro_density²*h
Q_VV = accel_density²*h*I
Q_VP = Q_PV = accel_density²*h²/2*I
Q_PP = (accel_density²*h³/4 + integration_density²*h)*I
```

This follows GTSAM's discrete held-measurement noise convention using continuous
densities, not an exact continuous stochastic solution. Isotropic acceleration noise
removes the rotation from its noise blocks. Whitening `L⁻¹`, for `P = L*Lᵀ`, is prepared
when publishing a factor; covariance is never replaced by independent R/V/P weights.

## Lifecycle

- Ordered arrivals extend cached deltas. Completed prefixes are not reintegrated every
  solve. Raw samples, interval caches, and graph factors retire with the window.
- The active partial interval reaches the newest source sample. That newest gyro has
  no following span yet, so one temporary instantaneous constraint stabilizes the tail;
  it is removed before the next solve when the reading enters an integral.
- Gaps over 20 ms are not integrated. Gyros without usable following spans retain
  instantaneous constraints. Missing/nonfinite/disabled acceleration creates rotation-only
  pieces; full pieces resume when force returns. Missing force is never replaced with zero.
- SDK up-direction evidence is one duration-weighted endpoint observation per bin.
  Partial intervals carry proportionally less information. Isolated gap readings retain
  the previous instantaneous tilt weight. The newest reading supplies tilt only if an
  interval does not already supply it. Dense attitude history remains available for
  exposure-time interpolation, independent heading validation, and recovery.
- Late/replaced readings invalidate affected spans. Calibration/noise changes invalidate
  active caches. Accepted bias changes exceeding 0.01 rad/s or 0.1 m/s² at either knot
  trigger reintegration between solves. The optimizer uses analytic first-order correction
  within a solve; rejected trial biases never become integration references.
- Recovery replays raw IMU data through the same insertion/preparation path. Bias rollback,
  joint marginalization, broad recovery confidence, heading references, and generation/epoch
  checks retain their semantics.

`imu_preintegration` defaults: gyro density `sqrt(2e-5)`, integration density `1e-4`,
complete-bin tilt sigma `0.02`, terminal gyro sigma `0.1`. Accelerometer density remains
`0.3`. The old `accelerometer.averaging_interval` is removed; fixed bias, scale, and
physical mounting position remain configurable. Active calibration changes rebuild
retained data but cannot recalibrate a marginalized prior; use a fresh epoch for a
physical calibration change.

SDK attitude remains correlated with raw IMU evidence. Covariance describes this
configured model, not independently calibrated physical uncertainty. The coarse spline
still limits representable motion bandwidth.

## Reference and regression checks

Reference sources: GTSAM 4.2 `ManifoldPreintegration.cpp`, `ImuFactor.cpp`,
`TangentPreintegration.cpp`, and `NavState.cpp` under its BSD license. Reproduce the fixture:

```sh
uv run --no-project --python 3.11 --with gtsam==4.2 --with 'numpy<2' \
  python docs/tooling/preintegration_reference.py
```

The wheel uses **tangent** preintegration, whose multistep rotation update differs from
manifold integration. The reference uses native single-sample PIMs, converts their
covariance to NavState's right-local chart using numerical derivatives, then composes
them using numerical derivatives of GTSAM NavState operations. Finally it rotates the
velocity/position error axes and permutes `[R,P,V]` to `[R,V,P]`. It does not assert
equality with the wheel's unconverted multistep covariance.

The 100 ms fixture includes nonzero bias, changing rotation, 1–4 ms sample periods,
and a force impulse. Mean agreement is within `1e-12`; the covariance difference
whitened by the Rust covariance is below `1e-7`. Separate finite differences test both
interpolated bias Jacobians. f32/f64 automatic differentiation covers both factor modes
and nonzero mounting offsets. A physical 4 rad/s offset-IMU test distinguishes rotation
about the Robot origin from spurious body translation.

Additional checks cover late-data cost/information equivalence, partial prefixes,
missing force/free fall, impulses, gaps/late bridges, calibration changes, bias boundaries
and reintegration, retirement, rollback/recovery, double flight, and allocation-free
factor evaluation after workspace preparation.

### Sustained fast-turn finding

A three-second 4 rad/s blind yaw turn evaluates camera poses 37 ms before each solve,
between 100 ms endpoints. The raw baseline reaches **6.351317°** maximum error: the
unchanged zero-rotation process prior biases sustained blind turns. Preintegration
initially exposed a separate failure at 1.65 s: cost evaluation accepted a rotation-log
seam that Jacobian evaluation rejected. All spline factors now check the same smooth
rotation domain for cost and Jacobians, allowing LM to reject that trial. No upstream
solver change was needed.

The final version completes the experiment at **6.335169°** maximum error with
positive-definite camera-time pose covariances. This preserves a baseline limit; it is
not high-accuracy blind-turn tracking. Replacing the zero-rotation prior needs a separate
observability/model comparison. The source-baseline probe remains in detached worktree
`/tmp/opencode/localization-preintegration-baseline`, based on `9660abcad`.

## Matched robot43 performance

Three sequential matched rounds: old→new, new→old, old→new. Same recording, solve cadence,
release settings, and live-system load; one benchmark process at a time. Process CPU
includes decoding/setup. Estimator wall time includes ingestion and solving. `perf stat`
task-clock/instructions distinguish actual work from scheduler delay.

Medians across three runs:

| Workload | Version | CPU s | Instructions, billion | Estimator wall s | Cycle p50 ms | Cycle p95 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 30 s stationary, 565 cycles | Batched raw | 18.291 | 117.891 | 25.097 | 40.789 | 76.235 |
| Same | Preintegrated | **3.541** | **18.144** | **4.849** | **7.114** | **18.166** |
| 6 s moving tracking, 121 cycles | Batched raw | 4.083 | 26.587 | 5.763 | 44.991 | 97.608 |
| Same | Preintegrated | **0.815** | **4.418** | **1.198** | **9.275** | **25.045** |

CPU reductions: 80.6% / 80.1%. Instruction reductions: 84.6% / 83.4%.
Stationary CPU runs: old 18.291, 18.234, 18.428 s; new 3.543, 3.539, 3.541 s.
Tracking CPU runs: old 4.104, 4.080, 4.083 s; new 0.833, 0.815, 0.802 s.
Wall tails remain scheduler-dependent; replay speed is not a hard real-time guarantee.

Both versions return **119/121 tracking field outputs**. Maximum position/rotation
error changes from **9.634 mm / 0.240922°** to **9.110 mm / 0.230144°**. Both return
**564/565 stationary local estimates**. The new replay has 563 gradient-tolerance
terminations and two accepted-motion `NoProgress` outcomes versus 565 gradient
terminations; both include one initial covariance failure.

### Sixty-second calibration and observability

Both versions return 1147/1148 estimates in every mode. Neither receives the measured
stationary bias or a contact/zero-velocity equality.

| Mode | Old max displacement m | New max displacement m | Old max after 10 s m | New max after 10 s m |
| --- | ---: | ---: | ---: | ---: |
| Fixed-zero bias, with VO | 0.612388 | 0.606696 | 0.612388 | 0.606696 |
| Learn with VO | 0.429948 | 0.428209 | 0.189796 | 0.185650 |
| Learn, remove VO at 20 s | 0.429948 | 0.428209 | 0.301348 | 0.232854 |
| Learn without VO from startup | 27.375967 | 27.299938 | 27.375967 | 27.299938 |

With VO, final positions differ by **2.365 mm**. Learned acceleration bias changes
from `[0.0121701, 0.0282008, 0.00167036]` to `[0.0121842, 0.0281974, 0.00166241]` m/s².
Marginal standard deviations change from `[0.044226, 0.044330, 0.042125]` to
`[0.044641, 0.044701, 0.043444]` m/s². Covariance is not asserted equal to the old model.

Gradient-tolerance counts old/new: fixed-zero 1028/1030, learn-with-VO 1148/1146,
VO-outage 1128/1088, no-VO 1148/1146. The new outage run has 52 `NoProgress` and eight
`MaxIterations` outcomes; accepted decreasing-cost motion uses the existing validation
and covariance policy. Availability/drift did not regress, but gradient convergence
became less frequent during the outage.

No-VO startup still drifts about 27 m in this stationary minute: bias learning needs
independent motion evidence. The Sept 26 divergence window (08:19–08:36) produces
332/333 local estimates, zero field estimates, max absolute height 0.606 m, and zero
field resets; raw baseline: 330/333 and 0.605 m. This does not demonstrate field recovery
in that incident.

## Reproduction and artifacts

```sh
cargo test -p localization-fagra -p localization-3d
cargo test -p localization_simulator --lib
cargo test -p localization-3d recorded_divergence_window_remains_bounded_without_contact -- --ignored --nocapture
cargo clippy -p localization-fagra -p localization-3d -p localization_simulator --all-targets --no-deps -- -D warnings
target/debug/pepsi build crates/nodes/localization-3d --env podman --release \
  --example imu_calibration_replay --tests --config /hulk/target/localization-perf.toml
```

Passed: 153 factor/library tests, two allocation checks, one doctest, 28 node tests on
host/robot43, 42 simulator tests, and the explicit divergence replay. The old Sept 24
external recording and GPU-render test retain their existing ignores.

On robot43, wrap `replay-VARIANT RECORDING 30 learn-with-vo` or
`tests-VARIANT benchmark_tracking_estimation --ignored --nocapture --test-threads=1`
with `perf stat -x , -e task-clock,cycles,instructions,context-switches,cpu-migrations`.
For calibration use `replay-VARIANT RECORDING 60 all`. Recording:
`/home/booster/hulk/logs/2026-09-27T17:24:08.443+08:00/recording.mcap`.

Artifacts: robot `/tmp/localization-perf/`; host `/tmp/opencode/localization-preintegration/`
(including `summarize.py`). Final tables use only:
`preintegration-validated-{stationary,tracking}-{batched,preintegrated-validated}-{1,2,3}.*`,
`preintegration-validated-calibration-60.jsonl`, and `preintegration-calibration-batched-60.jsonl`.
Earlier experiments remain under distinct names.

| Executable | SHA-256 |
| --- | --- |
| `replay-batched` | `d4e8a3ae3cee7bb5d0bb07ffba2b65ece8c7368f1a97a471f65f8f8469bdd079` |
| `tests-batched` | `f11e24b4b2bbc1dd5f110f0caeb8440450e06d47a73cf00f6f97aef9108dbf81` |
| `replay-preintegrated-validated` | `e903589efeda89cd2fe967279c7b69b222f7473dbccdefbb3ee3b6145284c670` |
| `tests-preintegrated-validated` | `595d08cb38c1725df5c6214f41abed71e88a9e859eae674f5a585fb81449e713` |

## Review cleanup comparison

The follow-up cleanup removes `ForceBias`, `LeverArmSample`, the old accelerometer
residual/Jacobians, and `ImuObservation`. Instantaneous gyro/tilt is now an ordinary
`ImuKinematics` factor rather than a singleton batch; ordinary marginalization removes
its factors without separate batch cleanup. Tilt-only evaluation computes pose only.
Nonzero mounting offsets share pose/kinematics evaluation, and the state-Jacobian
assembly is reused. Interval preparation borrows raw samples and collects only gap
boundary descriptors. Bias-time mapping reuses the existing helper.

Delta rotations now use `Rotation3<Robot, Robot>` with the documented end-to-start
direction. Endpoint velocity/position arithmetic and SDK up conversion retain frame
types until residual/Jacobian matrix assembly. Preparation failures cannot take the
visual-rejection retry into a stale graph. A regression uses finite accepted readings
that overflow covariance propagation to verify that no optimization/retry occurs;
diagnostics retain the failing interval, timestamps, operation, and underlying error.

Three matched robot43 rounds compared the cleanup against `preintegrated-validated`,
alternating execution order, with the same workloads and perf counters as above:

| Workload | Version | Process CPU s | Instructions, billion | Estimator wall s | Cycle p95 ms |
| --- | --- | ---: | ---: | ---: | ---: |
| Stationary | Before cleanup | 3.808 | 18.151 | 6.526 | 23.210 |
| Stationary | Cleanup | **3.627** | **17.824** | 5.922 | 21.064 |
| Tracking | Before cleanup | 0.860 | 4.420 | 1.507 | 29.466 |
| Tracking | Cleanup | **0.836** | **4.358** | 1.516 | 30.503 |

Median CPU reductions are **4.75% / 2.85%**; instructions fall **1.80% / 1.40%**.
Scheduler noise obscures the smaller change in tracking wall time. These are matched
cleanup measurements, not ratios against the earlier runs under different load.

Both versions return 564/565 stationary estimates, with 563 gradient-tolerance and two
`NoProgress` terminations. Final position differs by `3.96e-12` m, acceleration bias by
`2.04e-14` m/s², and bias-covariance entries by at most `7.69e-14`. Tracking retains
119/121 field estimates and the same printed 9.110 mm / 0.230144° maximum errors.
The divergence replay retains 332/333 local estimates and 0.606 m maximum absolute height.

Checks pass: 153 factor/library tests, two allocation checks, the schema doctest, 29
node tests on host/robot43, 42 simulator tests, clippy with warnings denied, and the
explicit divergence replay. Artifacts:
`cleanup-{stationary,tracking}-preintegrated-{validated,cleanup}-{1,2,3}.*` in the same robot/host directories, with
`summarize_cleanup.py` on the host.

| Executable | SHA-256 |
| --- | --- |
| `replay-preintegrated-cleanup` | `7a7166a668db442ce407ae8013a6e635f082da93ebae37082feeae5b4d2acbbd` |
| `tests-preintegrated-cleanup` | `47e297bde26437dfe44a7e370fe3fe32e4d7b07850356dee637f02d31e6010d7` |
