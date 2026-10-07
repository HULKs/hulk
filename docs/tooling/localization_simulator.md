# Localization Simulator

## Purpose

Provide a deterministic, interactive environment for diagnosing localization drift, jumps, and
field-symmetry flips without running camera neural networks or robot hardware.

The simulator renders the SPL field, moves a ground-truth camera rig along a six-degree-of-freedom
trajectory, synthesizes localization inputs, runs the production association and localization
implementations, and displays estimated poses against ground truth.

## Localization Lifecycle

The robot publishes `localization/estimate` (timestamp, epoch, generation, local pose/covariance,
and optional field pose/covariance) and transient-local `localization/status`
(state-entry timestamp, epoch, generation, Startup/Tracking/LostTrack, and an optional trusted IMU-to-field
heading reference with its own timestamp). Localization owns that reference; association consumes
an epoch-matched snapshot. `debug/solve_diagnostics` reports each numerical attempt.

Bootstrap/recovery increments the generation without restarting the epoch. Estimates and visual
frames must match both identifiers; a new status cannot relabel a cached pre-recovery pose.
Tracking selects the newest admissible preceding pose, never a future anchor. The node and simulator's
production-association mode call the same sensor-attitude/lifecycle dispatcher. Known correspondences
bypass matching only: startup and recovery still use the same backend candidate path.

| State | Association | Transition |
| --- | --- | --- |
| `Startup` | Stateless global landmark-geometry matching | Three or more certified correspondences and an accepted backend result establish tracking. |
| `Tracking` | Image-space prediction and uncertainty-gated one-to-one assignment | Solver or visual freshness failure enters `LostTrack`. |
| `LostTrack` | Global landmark matching constrained by IMU-propagated field heading | A post-loss recovery candidate validated by the backend restores tracking. |

The own-half assumption (robot field X below zero) is used only when localization computes its
startup alignment seed. Tracking and recovery retain the established field orientation, including
in the opponent half. Leaving damping resets the estimator and starts a new startup epoch.
Association returns correspondences, not a fitted field pose. Localization alone seeds and
optimizes alignment. Startup association uses exposure-time IMU tilt and calibrated camera geometry,
without requiring a localization pose. Rays intersect a unit-height plane; guided two-point
hypotheses estimate camera height, planar position and yaw with IMU tilt fixed. Candidates are
scored and refined in pixel space. Camera extrinsics convert this to body height. The backend
also refines its alignment seed in pixel space before anchoring a candidate at image exposure,
replaying recent motion on the original knot grid before committing it. Sole kinematics seed the provisional
local trajectory before landmark initialization; that height is not used for global matching.

Localization and association consume `camera_geometry`: calibrated robot-to-camera transforms
and intrinsics stamped with head kinematics time. This stream remains available without a ground
transform or a selected support foot. Image-time lookups interpolate bracketing camera-to-body
poses across gaps of at most 20 ms, rejecting extrapolation and changes in intrinsics. Geometry
caches hold 1500 samples (three seconds at 500 Hz). Ground-projection consumers still use
`camera_matrix`, which requires a fresh ground transform. The simulator supplies the same
ground-independent camera geometry directly to localization.

Global matching samples at most 128 valid hypotheses from 2048 deterministic, rarity-guided
proposals on a single worker. Seeds use matching classes; scoring allows penalized L/T/X class
confusion while goalposts and penalty spots remain hard-class. Outliers can remain unmatched.
The default consensus requires 70% of confidence-filtered, deduplicated detections before
uncertain-ray pruning, at least four inliers when more than
three detections are present, non-collinear support, and pixel RMS at most 10. Camera height
is bounded by `min_camera_height`/`max_camera_height`; `height_sigma` remains a tracking-only
uncertainty floor. Near-tied sampled poses, ambiguous exact assignments at the winning pose,
and work exhaustion reject the frame. Sampling is not an exhaustive uniqueness proof.
Tracking propagates the full pose covariance into image space. Recovery uses exposure-time
IMU tilt and constrains hypotheses by trusted robot field heading before scoring and refinement.
It preserves the selected correspondence orientation instead of canonicalizing it to the opposite
half. Recovery does not need a fresh optimized position or an unexpired tracking prediction.
Repeated stationary images are retries, not independent votes for an ambiguous pose.

Pose-based tracking with three to five detections scores joint image residuals, including shared
pose uncertainty across features. The joint Mahalanobis cutoff preserves the configured 2D gate's
tail probability for the larger residual dimension. Larger frames use the faster marginal-assignment
path. Sparse
joint search is bounded by `global_localizer.max_work`; budget exhaustion rejects rather than
returning a winner before all plausible alternatives have been checked.

Association uses one blocking worker and one replaceable pending detection frame.
Localization drains a bounded snapshot of the ROSZ subscriber queues, solves its fagra
graph, publishes, and repeats. Queue capacities cover one second: 500 IMU, 500 kinematics,
60 visual-localization and 60 VO messages. Overflow drops the oldest sample with a warning.
No frontend mailbox or live-VO output propagation exists. Numerical work runs on a
blocking thread, while ROSZ continues receiving. Timestamped geometry caches cover
supporting lookups. Late measurements inside the two-second window remain eligible;
older measurements are warned and discarded. A measurement gap exceeding the whole
window reinitializes the local frame with a new epoch.

The trajectory uses four-control cubic spline segments, 200 ms spacing, and f64 geometry.
Velocity and acceleration are derived from position. Levenberg–Marquardt uses fagra's
`DenseNormalCholesky` backend, up to eight damping trials per step and ten optimizer iterations.
The pinned fagra revision is `7b37df61ed615c85f5d59265f5df53ff5d5dc1be`; no local solver
implementation or dependency patch is needed. Diagnostics include `lm_attempts`, `lm_rejected_steps`,
`gradient_norm`, and `motion_rebuilt`, alongside cost and termination. Covariance includes
control/alignment cross-correlations; numerical damping is not a physical observation.

For CPU hotspots and scheduling-delay measurements on robot 43, see the
[perf profiling report](localization_perf.md).

The estimator now reuses LM's final undamped linearization for covariance and takes costs
from solver reports/statistics instead of running extra evaluation-only solves. Accepted
nonconverged motion results still get a fresh covariance. `initial_cost` is unavailable on
failures after accepted steps when fagra does not return a report; final cost and termination
remain available. Exactly inactive foot
constraints and zero Jacobian blocks avoid assembly work without dropping measurements.
Gyro and force now use correlated 100 ms preintegrated endpoint factors, with partial
intervals through the newest sample. SDK tilt is reduced to coarse endpoint observations.
See the [preintegration model and comparison](localization_preintegration.md).
Small local spline optimizations borrow the tilt Jacobians and propagate only the already-active
angular derivative prefix. Their incremental measured impact is recorded in the
[local refinement results](localization_perf.md#local-refinements-complexity-versus-measured-gain).

Rejected visual updates are rolled back and removed before retrying the restored motion graph.
If the field-conditioned graph remains invalid, localization reconstructs local motion from an
owned validated pose/velocity/IMU checkpoint and retained observations, enters LostTrack, and keeps
the independent field-heading reference. If that checkpoint predates retained history, initialization
coasts to its left boundary with increasing height uncertainty. This bounded fallback does not
reconstruct discarded motion. Boundary controls are guesses, never extra observations. Decreasing-cost
motion-only partial solves may publish; visual updates and global bootstrap require convergence.

### Contact-independent inertial fusion

The SDK accelerometer measures **specific force**: approximately +9.81 m/s² on Robot Z upright
at rest, and zero in free fall. `accelerometer` configures Robot-frame bias, per-axis scale,
sensor position, and noise density. Calibrated force is preintegrated at the physical IMU
origin; the factor transforms spline endpoint position and velocity using the mounting offset.
This avoids angular-acceleration differencing. Zero force and impulses remain observations.
Missing acceleration produces rotation-only pieces; gaps over 20 ms are not integrated.
Calibration changes rebuild retained intervals from raw data.

Bundled settings enable acceleration with 100 ms preintegration and noise density 0.3, but zero bias/position
and unit scale are provisional: the physical mounting offset and calibration have not been measured.
The simulation IMU site's origin is not hardware calibration. The 200 ms spline also limits the motion
bandwidth represented by the trajectory between endpoint observations.

### Online IMU bias calibration

The graph estimates residual accelerometer and gyroscope biases in Robot axes, after the
configured fixed calibration. Bias has independent **5-second knots with linear interpolation**,
not a 200 ms spline. Each observation references two bracketing six-dimensional bias states;
the two-second trajectory window normally retains two knots, or three across a boundary.

For interpolation fraction `u`, `b(t) = (1-u) b0 + u b1`. Preintegration subtracts
the interpolated gyro/force bias and propagates sensitivities for both reference knots.
Bias tangent/covariance order is gyro XYZ, then
accelerometer XYZ. The bias walk penalizes consecutive differences with variance `density² * 5 s`.

`localization3d.imu_bias` configures initial uncertainties and drift densities:

| Parameter | Default | Units |
| --- | ---: | --- |
| `accelerometer_initial_sigma` | 1.0 | m/s² |
| `gyroscope_initial_sigma` | 0.02 | rad/s |
| `accelerometer_random_walk` | 0.002 | (m/s²)/sqrt(s) |
| `gyroscope_random_walk` | 0.0002 | (rad/s)/sqrt(s) |

The initial bias mean is zero; the broad initial accelerometer prior allows uncalibrated offsets
to be learned. The knot interval is structural, like the pose-knot spacing. Parameter changes
affect newly constructed priors/walks, not already marginalized information.
Fixed sensor bias/scale/mounting calibration is assumed constant within an epoch; start a
fresh epoch after changing those physical calibration parameters.

Intervals split at bias boundaries without dropping intervening impulses. The optimizer uses
first-order bias correction with analytical Jacobians; accepted changes exceeding configurable
thresholds trigger reintegration between solves. Mounting offsets enter endpoint kinematics,
so no gyro-bias time derivative is needed for sensor-origin integration.

Bias knots retire through joint marginalization with pose controls, retaining cross-correlations.
Rejected updates restore bias along with pose/alignment/intrinsics. Recovery candidates start
from the existing bias estimate but reset its confidence using the original zero-mean calibration
prior, broadened for elapsed drift; they do not duplicate an active posterior alongside replayed
measurements. Full epoch resets start calibration anew.

`debug/solve_diagnostics.imu_bias` exposes the estimated gyro/accelerometer offsets and their
interpolated 6×6 marginal covariance (array of rows), at the diagnostic's estimate timestamp.
It is absent when no estimate was accepted. Rebuild diagnostic consumers for this schema change.

Bias requires independent motion evidence, such as VO or landmarks; it is not identifiable from
IMU alone at startup. No stationary/contact detector, zero-velocity factor, or standing-height
constraint is introduced. Scale, mounting geometry, and the SDK attitude filter are not optimized.
See [robot 43 calibration results](localization_robot43_investigation.md#online-calibration-validation)
for the measured learning transient, VO outage, and unobservable-startup behavior.

Kinematic odometry is optional and disabled in the bundled configuration because there is no reliable
contact signal. Foot factors enforce nonpenetration only; both feet may be airborne. The anchor fixes
Local XY/yaw gauge, with broad initial velocity/height uncertainty, not a standing-height or tilt
equality. Visual bootstrap adds no duplicate height prior. Full-vector IMU-up residuals distinguish
upright from inverted poses, and the final tilt check uses `max_tilt_error`.

When a converged aligned source-time height marginal is available, recovery compares candidate and
active heights using their summed variance and `recovery_height_gate`. This is an acceptance check,
not another prior; it is skipped when the active marginal is unavailable. It imposes neither contact
nor an altitude ceiling.

The simulator defaults to production association and supplies estimator geometry, not ground-truth
poses, to that path. Known-correspondence mode remains available explicitly for estimator isolation.
Its synchronous runner shares the production loss and visual-acknowledgement checks.

### Live Tuning

Visual factors use a [monotone oriented-bearing residual](localization_visual_residual.md),
including outside the image and behind the predicted camera. The existing pixel-noise
parameter is converted to fixed isotropic angular noise at insertion; final tracking
acceptance still checks positive depth and pixel reprojection RMS. See the linked
contract for the formula, derivatives, monotonicity proof, and antipodal limitation.

The `localization3d` parameter API updates the running node without resetting its pose, alignment,
or epoch. Timeout changes take effect immediately, including shortening an already armed deadline.
Both tracking timeouts must be positive and no greater than 24 hours. Numerical validation is
shared with the backend and runs before remote parameter changes are committed.

Noise and containment updates enter the backend's measurement channel without blocking ingestion.
The latest update in a drained batch applies before constructing factors in that batch. Existing
factors, including extended interval factors, keep their original weights until they leave the
optimization window. Marginalized information is not retrospectively reweighted. Configuration-only
batches do not trigger a solve, and damping resets preserve the latest tuning. Knot spacing, window
size, and iteration limits are structural settings and are not exposed as live node parameters.

Association parameters are sampled per frame. Geometry messages no longer contain a redundant
source tag; the epoch and localization state determine how they are used. Global diagnostics expose
`association_count` and `pairwise_distance_rms`, not an apparent candidate score that merely repeated
the count. Rebuild robot and visualization consumers together after these message-schema changes.

Run solver-only host timing characterization with:

```sh
cargo test -p field_mark_association --lib runtime_characterization -- --ignored --nocapture
```

The measurements exclude transport, perception and backend scheduling. Measure end-to-end frame
age and tail latency on the robot before treating them as deployment performance claims.

### Sparse-Feature Regression

```sh
cargo test -p localization_simulator --lib sparse_ -- --nocapture
```

These tests restrict emitted sensor observations, then run production association and the real
backend with the bundled parameters. They do not inject correspondences, pose hints, or fabricated
backend acknowledgements. They cover:

- Startup with zero, one, and two features, followed by stationary three-feature acquisition.
- A stationary field-boundary view containing only two L spots and one penalty spot, including
  loss and recovery.
- A short two-feature gap bridged while tracking, and a longer gap causing loss at the visual deadline.
- Recovery with three, four, and five features, including in the opponent half without a symmetry reset.
- Five deterministic noisy stationary runs (0.5 px landmark noise and 1 mm / 0.001 rad VO noise per step).
- Rejection of a frame at the exact loss boundary until a post-loss frame is acknowledged.
- Heading-guided recovery after the old tracking prediction's five-second validity horizon.

The tests require a live global pose only during Tracking, preserve the last trusted estimate while
lost, and check every published pose against truth within 10 cm and 5 degrees. Output prints actual
state-transition times and maximum errors. Synthetic geometry and sensor noise do not reproduce
neural-network misclassifications, camera calibration error, transport scheduling, or hardware latency.

### Recorded flip regression

The localization tests include source-time fixtures from `recovered.mcap` (association sequences
3734, 3735, 3737); their provenance and recording checksum are in
`crates/nodes/localization-3d/src/localization/recovered_frames.json`. The optional direct-file test
streams the MCAP, decodes the actual observations and IMU brackets, then checks production matching,
symmetry selection, candidate optimization and heading acceptance:

```sh
HULK_RECOVERY_MCAP=/path/to/recovered.mcap cargo test -p localization-3d recorded_flip_from_mcap -- --ignored --nocapture
```

This is an incident-frame regression, not a continuous replay of the complete robot pipeline.
Field dimensions are supplied by the recorded fixture; tuning comes from the repository defaults.

### Recorded divergence regression

The separate September 26 recording (SHA-256
`51598d35164a979b0476bbe05a463302c28be4bf70c1914b226b0b4a6c55dd1f`)
supports a controlled replay of elapsed 08:19–08:36:

```sh
HULK_RECOVERY_MCAP=/path/to/recovered.mcap cargo test -p localization-3d recorded_divergence_window_remains_bounded_without_contact -- --ignored --nocapture
```

This replays recorded IMU and visual odometry in delivery order, initialized from the recorded
pre-incident local pose and heading, and submits suspect global association 3722. It omits contact
and leg-odometry factors and supplies field dimensions; it is not a complete association-pipeline replay.
The latest host run produced 331/333 local estimates, maximum absolute height 0.605 m, and **zero field
estimates**. Thus it verifies bounded local motion, not successful field recovery. Runtime was about
77 seconds for 17 seconds of data, including recording reads; real-time deployment performance remains
unverified. The older direct flip test requires the original September 24 file, not this replacement.

### Whole-estimator benchmark on 10.1.24.99 (2026-09-26)

Release build of the then-current LM + LSMR estimator, running as an isolated test executable on the
robot: NVIDIA Jetson Orin NX, six Cortex-A78AE cores, MAXN_SUPER power mode, schedutil governor,
maximum CPU frequency 1.984 GHz. One full warm-up per workload, then three sequential measured
runs. The robot was initially idle. Power settings and the installed robot application were not changed.

Each cycle measures accumulated ingestion/factor construction plus the entire `Localization::solve`
call: optimization, validation, covariance, marginalization, retries, and recovery candidate work.
It excludes initial object construction, MCAP decoding, synthetic input generation, transport,
perception/association, publishing, and replay waits. Solves are scheduled at 50 ms sensor-time
intervals; replay runs as fast as computation permits, without reproducing live queue/backpressure
behavior. These are wall-clock processing measurements, not CPU-time measurements.

| Workload/run | p50 ms | p95 ms | p99 ms | max ms | Total processing s | Processing / sensor duration |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Synthetic tracking 1 | 112.294 | 243.488 | 262.188 | 264.040 | 14.584 | 2.431 |
| Synthetic tracking 2 | 112.592 | 244.039 | 262.644 | 263.925 | 14.599 | 2.433 |
| Synthetic tracking 3 | 112.803 | 244.098 | 262.630 | 264.564 | 14.619 | 2.436 |
| Recorded local motion 1 | 705.768 | 849.792 | 918.100 | 1153.271 | 209.088 | 12.299 |
| Recorded local motion 2 | 705.782 | 848.864 | 917.246 | 1152.465 | 208.941 | 12.291 |
| Recorded local motion 3 | 707.184 | 850.435 | 918.653 | 1154.053 | 209.358 | 12.315 |

- Synthetic tracking: six sensor seconds, 121 cycles, 500 Hz IMU, 50 Hz exact VO, 10 Hz three-landmark
  correspondences, moving/turning trajectory, bundled acceleration settings, no contact/leg factors.
  Each run published 119 field estimates and reported one failed solve. Includes initialization.
- Recorded local motion: the 17-second incident fixture above, 333 cycles, 331 local estimates,
  two failed solves, zero field estimates, no field resets; max absolute height remained 0.605 m.
- Input ingestion contributed only 5–6 ms **in total** per synthetic run and 17–18 ms per recorded
  run. Nearly all measured time is inside the complete estimation call. This measurement does not
  isolate the linear solver from factor evaluation, covariance, or marginalization.

Neither workload meets its 50 ms cycle budget. The respective average processing capacities are
about 8.3 and 1.6 cycles/s, versus the requested 20 cycles/s. The ratios above are real-time cost,
not a slowdown relative to an older estimator.

Diagnostics now expose `estimation_duration` (complete `Localization::solve`, including discarded
candidates) and `ingestion_duration` (filled by the live node around input ingestion). Sum them for
whole-cycle estimator time. Existing `duration` retains its narrower selected-estimator-attempt
meaning. Other callers must fill `ingestion_duration` themselves; by default it is zero.

Reproduce the cross-build and run the resulting test executable with these filters:

```sh
target/debug/pepsi build crates/nodes/localization-3d --env podman --release --tests
# On the target, with the copied aarch64 release test executable:
./localization-bench benchmark_tracking_estimation --ignored --nocapture --test-threads=1
HULK_RECOVERY_MCAP=/path/to/recovered.mcap ./localization-bench recorded_divergence_window_remains_bounded_without_contact --ignored --nocapture --test-threads=1
```

The measured executable remains at `/tmp/localization-bench-20260926/localization-bench` on the
target, SHA-256 `cfd292ce4f8684cebadff7d07dd63c330c8320ca37f32dcbca2b28e3dcc2ea84`.
The recording is in the same directory. The benchmark reports nearest-rank percentiles over all
cycles, including failed solves and initialization; it does not discard slow cycles.

### Gauss–Newton comparison (2026-09-27)

Repeated the same two workloads on `10.1.24.99`, with the same release cross-build, MAXN_SUPER
power mode, factor weights, validation/rollback, covariance, and marginalization. Each variant had
one full warm-up followed by three measured runs. Each measured round ran LM+LSMR, GN+LSMR,
then GN+dense sequentially. The LM baseline was remeasured rather than using yesterday's times.

Both methods used max 10 nonlinear iterations, gradient tolerance `1e-3`, step tolerance `1e-5`,
and cost tolerance `1e-8`. LSMR retained block preconditioning, max 200 iterations, relative
tolerance `1e-6`. The direct variant used `GaussNewton::default()` (`DenseNormalCholesky`).
There is an important API distinction: this fagra revision's GN can declare convergence on step
or cost tolerance, whereas LM requires the gradient criterion and reports stalled progress as
`NoProgress`. These are operational comparisons of the supplied solvers, not equal-accuracy
linear-system microbenchmarks. Rejected whole updates are still rolled back in all variants.

Values below are medians of the three per-run statistics, except maximum, which is the worst
across all three runs. Total time includes every attempted cycle, successful or not.

| Workload | Solver | p50 ms | p95 ms | p99 ms | max ms | Total s | Published estimates | Failed solves |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 6 s tracking | LM + LSMR | 113.301 | 244.825 | 263.613 | 265.827 | 14.690 | 119/121 field | 1 |
| 6 s tracking | LM + dense Cholesky | 36.421 | 55.575 | 59.537 | 59.645 | 4.267 | 119/121 field | 1 |
| 6 s tracking | GN + LSMR | 215.590 | 909.035 | 930.558 | 937.472 | 37.887 | 57/121 field | 60 |
| 6 s tracking | GN + dense Cholesky | 35.246 | 40.614 | 41.510 | 41.627 | 3.473 | 119/121 field | 1 |
| 17 s incident | LM + LSMR | 707.507 | 851.573 | 917.630 | 1160.434 | 209.438 | 331/333 local | 2 |
| 17 s incident | LM + dense Cholesky | 100.500 | 126.514 | 142.610 | 181.202 | 31.074 | 329/333 local | 4 |
| 17 s incident | GN + LSMR | 182.159 | 350.492 | 516.254 | 910.740 | 64.127 | 32/333 local | 301 |
| 17 s incident | GN + dense Cholesky | 45.429 | 57.898 | 67.748 | 127.176 | 15.166 | 322/333 local | 11 |

Measured total seconds, in run order:

- Tracking LM: 14.677, 14.691, 14.690; GN+LSMR: 37.891, 37.879, 37.887;
  GN+dense: 3.472, 3.474, 3.473.
- Incident LM: 209.438, 209.586, 209.392; GN+LSMR: 64.138, 64.119, 64.127;
  GN+dense: 15.165, 15.166, 15.181.
- Follow-up LM+dense tracking: 4.271, 4.267, 4.263; incident: 31.074, 31.086, 31.064.
  These runs followed the original three-variant comparison on the same robot/power mode,
  with their own full warm-up per workload and three sequential measured repetitions.

GN+dense reduced total processing time by **4.23× in tracking** and **13.81× in the incident**.
Its real-time costs were 0.579 and 0.892 respectively. Tracking stayed below the 50 ms budget in
all measured cycles, but incident p95 exceeded it, with a worst cycle of 127 ms. Incident output
availability fell from 99.4% to 96.7%. No solver produced a field estimate in the incident fixture;
maximum published absolute height remained 0.605 m. This is bounded local-motion evidence, not a
field-recovery or incident ground-truth accuracy result.

Tracking accuracy was effectively identical for LM and GN+dense: maximum position errors
1.977 versus 1.973 mm, maximum rotation errors 0.069354 versus 0.069330 degrees. GN+LSMR's
small errors on the surviving 57 poses are not comparable coverage: it failed both benchmark
availability assertions. GN+dense also passed all 23 non-ignored localization tests on the robot.

The evidence favors the direct backend, but does not establish that removing damping is necessary:
GN+LSMR was 2.58× slower on tracking and rejected most incident outputs. LM with a direct backend
is measured in the follow-up below. Production source was restored to LM+LSMR after building the
experimental executables; only benchmark error reporting and this comparison were retained.

Executables and recording are in `/tmp/localization-solver-comparison/` on the target; use the
same test filters above. SHA-256 hashes:

| Executable | SHA-256 |
| --- | --- |
| `lm-lsmr` | `3afd96e23d55eccf06ff203ceb0e330bd7ff94c6a601c9dc0fc463d685c15631` |
| `localization-gn-lsmr` | `b89b5125cc891b8d2352d01336d4abbfcd4765d3c13d3fcb3ed784b48dc844ea` |
| `localization-gn-dense` | `80f5d7dbb00a03b2b20801257c6ce04789dce51e04e3e579f0901bbbc6ed0e0f` |
| `lm-dense` | `5805807c90ac13cc7667ab70bc780db3125a4b030ff85eccccd6bfbe92ca49f4` |

The GN builds temporarily replaced the estimator's optimizer type/constructor; LM-only diagnostic
statistics were zeroed, with max-iteration reporting used on GN nonconvergence. Those diagnostic
fields do not drive acceptance or benchmark timing. No optimizer runtime-selection API was added.

#### LM + dense follow-up

The pinned fagra commit lacked a damped direct backend. An isolated checkout at
`target/fagra-lm-dense` extends its existing `DenseNormalCholesky` to implement
`DampedLeastSquaresBackend`. It keeps the undamped normal matrix/RHS intact, copies them into
reusable trial buffers, adds `lambda * max(diag(JᵀJ), min_column_norm²)` to the diagonal, and
uses the existing sequential faer Cholesky kernels. Predicted reduction is evaluated against
the original undamped quadratic. Covariance/marginalization remain undamped. The optimizer
itself, damping policy (eight trials), convergence criteria, factors, and acceptance checks are
unchanged from LM+LSMR.

An independent augmented-QR reference test checks the direct steps and predicted reductions,
repeated/reordered damping trials, an unobserved coordinate, and preservation of the original
gradient. All 24 non-ignored fagra library tests passed. The LM+dense executable passed all
23 non-ignored localization tests on the robot as well as both timing regressions.

LM+dense is **3.44× faster on tracking** and **6.74× faster on the incident** than LM+LSMR.
Tracking errors remain effectively identical (1.977 mm max position, 0.069351° max rotation).
Compared with GN+dense, it costs 1.23× as much tracking time and 2.05× as much incident time,
but retains 329/333 incident estimates (98.8%) rather than 322/333 (96.7%). Its incident real-time
cost is still 1.828, so it does not meet the requested 20 Hz workload. Tracking average real-time
cost is 0.711, but p95 exceeds 50 ms. These results support dense LM as a speed/availability
compromise, not a demonstrated real-time solution for the incident workload.

This backend is experimental and lives in the isolated checkout, not the production dependency.
The benchmark used a CLI Cargo patch via `target/fagra-dense-config.toml` (container path
`/hulk/target/fagra-lm-dense`) and temporarily selected
`LevenbergMarquardt<DenseNormalCholesky>` in the estimator. Production source and dependency
selection have been restored to LM+LSMR. The measured `lm-dense` executable remains alongside
the other three on the robot.

#### Production adoption of upstream LM + dense

Localization now uses `LevenbergMarquardt<DenseNormalCholesky>` directly from fagra revision
`7b37df61ed615c85f5d59265f5df53ff5d5dc1be`. The local compatibility backend and its direct
faer/faer-ext dependencies were removed. The existing LM controls and acceptance checks remain.
The earlier solver comparison above records the experimental implementations used at that time.

A release-build confirmation on `10.1.24.99` passed all 23 localization tests and both replay
checks. One timing run per workload (not a new three-run comparison) gave:

| Workload | p50 ms | p95 ms | p99 ms | max ms | Total s | Estimates | Failed solves |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 6 s tracking | 36.345 | 55.478 | 59.434 | 59.462 | 4.257 | 119/121 field | 1 |
| 17 s incident | 100.246 | 126.136 | 142.258 | 180.567 | 30.993 | 329/333 local | 4 |

These agree with the experimental LM+dense results. The executable is
`/tmp/localization-solver-comparison/lm-dense-upstream` on the robot. Host validation passed
189 unit tests across localization, factors, and simulator, plus two allocation checks and Clippy.

## Requirements

### Correctness

- Use the current `FieldDimensions::SPL_2025` geometry, production semantic landmark map,
  stateless field-mark associator,
  localization configuration, factor-graph frontend/backend, global lock, and live visual-odometry
  propagation where applicable.
- Keep transform directions explicit and use typed coordinate-system transforms at boundaries.
- Generate the frame-to-frame visual-odometry delta and cumulative visual odometer from the same
  noisy measurement.
- Run simulation on a fixed logical clock. Rendering frame rate and playback speed must not alter
  generated inputs or localization results.
- Use independent, seeded random-number streams for each simulated sensor.
- Recreate and replay localization state when restarting. History inspection must not mutate the
  estimator.
- Record raw backend, live-propagated, and ground-truth poses separately.

### Simulation

- Support six-degree-of-freedom camera trajectories represented by timestamped positions and unit
  quaternions.
- Include straightforward built-in trajectories for stationary and moving tests.
- Provide a free-fly controller for exploratory movement and allow recorded motion to be replayed.
- Generate configurable visual-odometry translation/rotation noise and bias.
- Generate configurable field-mark pixel noise and dropout.
- Support deterministic one-shot visual-odometry outliers.
- Offer two field-mark modes:
  - known ground-truth correspondences, to isolate the estimator;
  - production field-mark association from class-grouped synthetic pixel detections.

### User Interface

- Render the field, ground-truth camera pose, backend estimate, live estimate, and their trails.
- Provide play, pause, single-step, restart, playback-speed, trajectory, association-mode, noise,
  and random-seed controls.
- Show simulation time, position/orientation errors, global-lock state, optimizer status, and factor
  residual summaries.
- Keep controls and colors documented in the application itself.

### Testing

- The simulation core must run without a renderer.
- Repeated runs with the same scenario and seed must produce identical results within floating-point
  tolerance.
- Include stationary, six-degree-of-freedom, visual-odometry-outlier, and association-mode tests.

## Non-Goals

- Photorealistic image rendering, occlusion, or neural-network execution.
- Robot dynamics, joint-level motion, or contact simulation.
- Foot-height factors. They are intentionally omitted so visual localization and odometry can be
  evaluated independently; add them only if a concrete localization experiment requires contact
  constraints.
- A generic ROS or MCAP replayer.
- Reproducing nondeterministic operating-system scheduling of the live ROS node.

## Launch and controls

Launch the native viewer from the repository development environment:

```sh
nix develop --command cargo run -p localization_simulator
```

Use the left panel to select a scenario and sensor settings, then press **Apply configuration /
rebuild**. Playback always advances the estimator in fixed 20 ms steps; the speed control only
changes how quickly those steps are requested. While paused, the history slider inspects recorded
samples without changing estimator state.

The collapsible **VO bias and one-shot outlier** section configures per-step translation/rotation
bias and a deterministic transition-indexed SE(3) outlier. Input edits are staged: playback is
disabled until **Apply configuration / rebuild** is pressed, preventing old results from being
mistaken for the newly displayed settings.

These perturbations apply only to synthetic VO. Production-stereo mode disables and clears those
controls; configuration files combining production stereo with synthetic perturbations are rejected
rather than silently running a different experiment.

The `pose_teleport` scenario changes the ground-truth robot pose discontinuously. The `vo_fault`
scenario remains physically smooth and enables a deterministic VO-only transform outlier. This
keeps robot relocation and sensor corruption independently testable.

The 3D view uses left-drag to orbit, right-drag to pan, and the mouse wheel to zoom. Cyan is truth,
yellow is the raw backend estimate, and magenta is the live estimate.

Flight recording is available while paused. **W/S** move along camera-local z, **A/D** along
camera-local x, **Q/E** vertically in the field, arrow keys control yaw and pitch, and **Z/C** control
roll. Stopping creates and selects a deterministic Custom scenario. The path field and Load/Save
buttons read and write that scenario as position/quaternion JSON5 keyframes.

## Headless analysis

Run the same deterministic simulator without opening a window and write a complete JSON report:

```sh
nix develop --command cargo run -p localization_simulator -- headless \
  --scenario six-dof-loop --output localization-report.json
```

Use `--scenario-file path.json5` for a custom trajectory and `--config path.json5` for complete
sensor settings. `--seed` and `--association known-correspondences|production-association` override
those individual settings. Without `--output`, the report is written to standard output. The
`vo-fault` preset installs its standard one-shot VO outlier unless a configuration file is supplied.
Run `localization_simulator headless --help` for all built-in scenario names and options.

Report schema version 5 includes the effective scenario, sensor configuration, SPL field dimensions,
and bundled production localization and association parameters. Every 20 ms sample contains:

- Timestamp in nanoseconds.
- Truth, raw backend, live, and cumulative noisy odometry SE(3) poses with explicit frame directions.
- Synthetic IMU values and every timestamped noisy VO transition passed to the frontend.
- Translation in meters and quaternion rotation in `[x, y, z, w]` order.
- `estimate_time_ns` and `truth_at_estimate_robot_to_field`, alongside current-tick truth.
- Backend and live translation and rotation error against truth at the estimate timestamp.
- Emitted semantic landmark pixels and accepted pixel-to-field correspondences.
- Global visual-lock state, visible/emitted/associated landmark counts, and solve diagnostics.

The summary counts each estimate timestamp once, so held poses do not duplicate accuracy samples.
It includes lock acquisition time, RMS/max/final pose error, and maximum consecutive pose
step for detecting jumps. Reports contain the complete timeline rather than only the summary, so
analysis scripts can derive additional metrics without rerunning the simulation.

Version 2 removes `landmark_frame.backend_reset_robot_to_field` and
`landmark_frame.associations[].source`. Consumers of version 1 reports must branch on the top-level
schema version before decoding landmark frames.

Version 5 also changes error semantics from current-tick truth to estimate-time truth. Consumers
of older reports cannot reconstruct estimate-time accuracy because those reports omitted the pose
timestamp. Staleness in version 5 is `time_ns - estimate_time_ns`.

## Scenario format

Positions are meters in the field frame. Each pose maps camera coordinates into the field frame,
and quaternions use `[x, y, z, w]` order. Timestamps are seconds, must be strictly increasing, and
must span exactly from zero to a duration aligned to the 20 ms simulation tick. Scenarios are
limited to 600 seconds.

```json5
{
  name: "short_forward_motion",
  duration_seconds: 1.0,
  camera_to_field_keyframes: [
    {
      time_seconds: 0.0,
      position: [-3.0, 0.0, 0.55],
      quaternion_xyzw: [-0.5, 0.5, -0.5, 0.5],
    },
    {
      time_seconds: 1.0,
      position: [-2.5, 0.0, 0.55],
      quaternion_xyzw: [-0.5, 0.5, -0.5, 0.5],
    },
  ],
}
```

Run deterministic headless coverage with:

```sh
nix develop --command cargo test -p localization_simulator
```

For a manual smoke test, run the viewer, step and restart a built-in trajectory, compare both
association modes, inspect earlier history while paused, and enable a one-shot VO outlier.

## Tasks

- [x] Expose the production semantic landmark list without duplicating field geometry.
- [x] Share the concrete fagra graph and lifecycle between the ROSZ node and deterministic runner.
- [x] Implement trajectory interpolation and built-in scenarios.
- [x] Implement deterministic synthetic camera, IMU, visual-odometry, and field-mark measurements.
- [x] Implement known-correspondence and production-association modes.
- [x] Ingest and solve each simulation input tick; retain original estimate timestamps in history.
- [x] Add a Bevy field scene with truth/backend/live markers and trails.
- [x] Add free-fly recording, playback controls, and diagnostics UI.
- [x] Add headless deterministic regression tests.
- [x] Document launch and scenario-authoring commands.
