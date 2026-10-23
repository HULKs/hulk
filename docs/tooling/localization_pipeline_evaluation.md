# 3D Localization Pipeline Evaluation

- Evaluation date: 2026-08-30
- Repository revision: `48e1eb3881404910a3870da2a0de8f5f36469de6`
- Simulator report schema: 2

## Executive summary

The 3D localization pipeline is fast enough to run comfortably faster than real time in the
headless simulator, and its visual-odometry fusion is accurate when landmark correspondences are
correct. It is not robust enough, on the evidence in this report, to describe production global
localization as reliably accurate.

The decisive result is a field-symmetry failure in production feature association. On the built-in
six-degree-of-freedom loop with default noise and seeds 0 through 9:

- All 10 runs reported global visual lock.
- Only 4 of 10 runs locked to the correct field pose.
- The other 6 runs locked to the 180-degree symmetric field pose.
- Failed runs ended approximately 6 m and pi rad from truth.
- Production association accepted 98.71% of emitted detections, but only 43.83% of accepted
  correspondences matched the simulator's known ground-truth correspondences across the batch.
- The lock state therefore cannot currently be interpreted as evidence that the global pose is
  correct.

When the exact same observations bypassed production association and used known correspondences,
all 10 runs were accurate:

- Mean live translation RMS: 7.61 mm.
- Mean live rotation RMS: 2.29 mrad, or 0.131 degrees.
- Mean final translation error: 3.12 mm.
- Mean final rotation error: 0.088 mrad, or 0.0051 degrees.
- Worst live translation error in an individual run: 26.5 mm.

This contrast localizes the dominant simulated failure to global feature association, not to live
VO propagation or factor-graph fusion.

A catastrophic one-shot VO fault was also tested with known landmark correspondences: 5 m of
translation and 170 degrees of rotation at transition 50. The translation error immediately rose
to about 4.91 m, then returned below 0.1 m and stayed there 180 ms after the faulty transition in
all 10 seeds. Final mean translation error was 3.00 mm. This is good recovery, but poor rejection:
the outlier was allowed to appear directly in the live pose until the next visual correction.

The optimizer frequently exhausted its configured iteration budget. `MaxIterations` occurred on
62.9% of solve checkpoints in the default known-correspondence runs, 61.9% in default production
association, 92.9% under association stress, and 97.6% with 1 cm / 0.01 rad per-step VO noise.
Bounded pose error despite this status does not make the status harmless; convergence and runtime
headroom require separate investigation.

## Bottom-line assessment

| Property | Assessment | Evidence |
|---|---|---|
| Relative-motion fusion with correct landmarks | Strong in simulation | 7.61 mm translation RMS over 10 noisy six-DoF runs |
| Final accuracy with correct landmarks | Strong in simulation | 3.12 mm and 0.0051 degrees mean final error |
| Production global feature association | Unreliable on the tested loop | 6/10 seeds selected the symmetric field pose |
| Lock-state trustworthiness | Unsafe as a correctness indicator | 10/10 locked, only 4/10 were globally correct |
| Moderate VO noise robustness | Pose can remain bounded in correct basin, solver health poor | 97.6% `MaxIterations` at 1 cm / 0.01 rad per step |
| Small persistent VO bias | Correctable when association is correct | Millimetric correct-basin results, but symmetry dominates aggregate |
| Catastrophic VO outlier rejection | Weak | Approximately 4.91 m live excursion |
| Catastrophic VO outlier recovery | Strong with correct landmarks | Stable below 0.1 m after 180 ms in 10/10 runs |
| Heavy landmark degradation | Partial | 9/10 acquired lock; only 34.5% of emitted detections associated |
| Determinism | Strong | Repeated report hashes were byte-identical |
| Headless simulator speed | Strong on test host | 9.0x to 11.5x simulated real time including JSON output |
| Real stereo VO speed and matching quality | Not measured | Simulator does not render images or execute XFeat/LightGlue |
| Deployment readiness | Not established | No camera model errors, neural inference, occlusion, motion blur, or hardware timing |

## Scope and terminology

"Feature association" can refer to two distinct pipeline stages in this repository:

1. Stereo visual-odometry feature matching. The production VO node uses a fused XFeat/LightGlue
   ONNX model to extract up to 512 keypoints and produce current-left/current-right stereo matches
   plus previous-left/current-left temporal matches. Stereo matches are disparity-filtered and
   triangulated. Temporal correspondences feed EPnP, RANSAC when needed, and LM refinement.
2. Semantic field-mark association. Detected goalposts, L spots, T spots, X spots, and penalty
   spots are associated with the 31-landmark SPL 2025 field map to provide absolute global
   localization constraints.

The localization simulator measures the second stage with production code. It does not measure the
first stage. Instead, it synthesizes an SE(3) VO delta directly from ground-truth motion and adds
configured noise, bias, or a one-shot outlier. Therefore:

- Claims about VO in this report apply to VO ingestion, factor construction, optimization, and
  live odometry propagation.
- Claims about association accuracy apply to semantic field-mark association.
- No measured claim is made about XFeat detector repeatability, LightGlue match precision/recall,
  stereo triangulation yield, PnP success rate on images, TensorRT/ONNX inference latency, or real
  camera frame rate.

This distinction is essential. The simulator is an estimator and association harness, not an
end-to-end visual-perception benchmark.

## System under test

The headless simulator runs the production components that matter after image-space features exist:

- SPL 2025 field geometry and the 31-point production semantic landmark map.
- Production stateless field-mark associator.
- Production localization parameters.
- Factor-graph frontend and backend.
- Global visual lock.
- Live cumulative visual-odometry propagation.
- Synthetic IMU, semantic pixel detections, VO deltas, and cumulative odometer.

The logical schedule is deterministic:

| Operation | Period | Frequency |
|---|---:|---:|
| Simulation and VO update | 20 ms | 50 Hz |
| Semantic landmark frame | 100 ms | 10 Hz |
| Backend solve | 200 ms | 5 Hz |

Independent ChaCha8 random streams are derived from each run seed for VO and landmark noise.
Changing landmark noise does not perturb the VO random sequence and vice versa.

## Test environment

| Item | Value |
|---|---|
| OS | NixOS, Linux 7.2.0 x86-64 |
| CPU | AMD Ryzen 9 9900X, 12 cores / 24 threads |
| L3 cache | 64 MiB |
| Build | Cargo `--release`, optimized |
| Execution | Native headless process, compact JSON report |
| Renderer | Disabled |
| Neural network | Not executed |
| Dataset | Deterministic built-in synthetic trajectories |

Timing results are host-specific and include process startup, simulation, production association,
factor-graph work, warning logging, complete report construction, and compact JSON serialization.
They exclude image decoding, camera capture, neural inference, and ROS/Zenoh scheduling.

## Experiment matrix

The main six-DoF scenario lasts 4 seconds and contains translation, height change, roll, pitch, and
yaw. Each stochastic row was run with seeds 0 through 9.

| Experiment | Association | Landmark sigma | Dropout | VO translation sigma | VO rotation sigma | VO bias per step | Runs |
|---|---|---:|---:|---:|---:|---|---:|
| Exact isolation | Known and production | 0 px | 0% | 0 m | 0 rad | zero | 1 each |
| Default baseline | Known | 0.5 px | 0% | 1 mm | 1 mrad | zero | 10 |
| Default production | Production | 0.5 px | 0% | 1 mm | 1 mrad | zero | 10 |
| Association stress | Production | 3 px | 50% | 1 mm | 1 mrad | zero | 10 |
| VO noise stress | Production | 0.5 px | 0% | 10 mm | 10 mrad | zero | 10 |
| VO bias | Production | 0.5 px | 0% | 1 mm | 1 mrad | 0.5 mm x and 0.5 mrad yaw | 10 |
| VO catastrophic fault | Known and production | 0.5 px | 0% | 1 mm | 1 mrad | 5 m and 170 degrees once | 10 each |
| Field figure-eight twice | Known and production | 0.5 px | 0% | 1 mm | 1 mrad | zero | 1 each |
| Stationary | Known and production | 0.5 px | 0% | 1 mm | 1 mrad | zero | 1 each |

The bias row injects bias every 20 ms. Its nominal rate equivalent is 0.025 m/s translation and
0.025 rad/s, or 1.43 degrees/s, yaw.

## Accuracy results

### Known correspondences: estimator and VO fusion isolation

The known-correspondence mode answers the question: if semantic detections are assigned to the
correct field points, how accurately does the localization stack fuse landmarks and noisy VO?

| Metric across seeds 0-9 | Mean | Best run | Worst run |
|---|---:|---:|---:|
| Live translation RMS | 7.61 mm | 6.57 mm | 8.79 mm |
| Live rotation RMS | 2.29 mrad | 1.92 mrad | 2.71 mrad |
| Final translation error | 3.12 mm | 0.081 mm | 4.93 mm |
| Final rotation error | 0.088 mrad | 0.008 mrad | 0.178 mrad |
| Maximum translation error | 23.5 mm | 18.6 mm | 26.5 mm |
| Lock acquisition | 0.8 s | 0.8 s | 0.8 s |

All 10 runs produced a live estimate and acquired lock. All 6,366 emitted detections were passed as
known associations. The simulator emitted 99.94% of ideally visible landmarks; the small loss is
caused by pixel noise moving points out of image bounds.

These results establish a useful upper bound for this synthetic trajectory. They demonstrate that:

- Transform directions are internally consistent under six-DoF motion.
- No material estimator lag remains after live VO propagation.
- Default 1 mm / 1 mrad independent per-step VO noise is well controlled by periodic absolute
  visual constraints.
- Final error is millimetric once the global basin is correct.

They do not establish that real VO deltas have the simulated noise distribution.

### Exact-noise isolation

With all synthetic noise disabled, known correspondences ended at 0.55 mm translation error and
0.203 mrad rotation error in the raw backend, while the live output ended at 0.368 mm and 0.050
mrad. The live path's RMS translation error was 6.38 mm.

The nonzero transient and RMS error in an exact-input run reflect initialization and optimization,
not sensor noise. Consequently, simulator-wide RMS values should always be interpreted with lock
acquisition and startup transients in mind.

### Production association: bimodal global accuracy

The production-association result is not a broad Gaussian error distribution. It is bimodal: four
runs are excellent and six are catastrophically mirrored.

| Seed | Lock time | Final translation error | Final rotation error | Classification |
|---:|---:|---:|---:|---|
| 0 | 0.6 s | 6.0037 m | 3.1416 rad | Symmetric-field failure |
| 1 | 0.8 s | 0.0021 m | 0.0001 rad | Correct |
| 2 | 0.6 s | 6.0045 m | 3.1416 rad | Symmetric-field failure |
| 3 | 0.6 s | 5.9954 m | 3.1415 rad | Symmetric-field failure |
| 4 | 0.8 s | 0.0009 m | 0.0002 rad | Correct |
| 5 | 0.6 s | 6.0022 m | 3.1415 rad | Symmetric-field failure |
| 6 | 0.6 s | 6.0007 m | 3.1415 rad | Symmetric-field failure |
| 7 | 0.6 s | 6.0035 m | 3.1415 rad | Symmetric-field failure |
| 8 | 0.8 s | 0.0019 m | 0.0001 rad | Correct |
| 9 | 0.8 s | 0.0048 m | <0.0001 rad | Correct |

The failed pose is coherent and stable, not numerically random. Approximately 6 m and pi rad is
consistent with choosing the opposite orientation on a symmetric field. Failed runs generally
locked 200 ms earlier than correct runs, so fastest lock was negatively correlated with correctness
in this sample.

Aggregate means obscure this mode split:

| Aggregate production metric | Value |
|---|---:|
| Mean live translation RMS | 2.877 m |
| Mean live rotation RMS | 1.885 rad |
| Mean final translation error | 3.602 m |
| Mean final rotation error | 1.885 rad |
| Lock success | 10/10 |
| Globally correct result | 4/10 |

The correct conclusion is not "about 3.6 m average error." It is "roughly 40% correct and 60%
catastrophically mirrored under this seed sweep."

### Long field-spanning motion

The 24-second field-figure-eight scenario makes two laps across approximately x = +/-4 m and
y = +/-2.5 m, with yaw following the path tangent.

| Mode | Translation RMS | Rotation RMS | Maximum translation | Final translation | Final rotation |
|---|---:|---:|---:|---:|---:|
| Known correspondences | 39.1 mm | 5.81 mrad | 294 mm | 3.32 mm | 0.409 mrad |
| Production association | 117 mm | 10.7 mrad | 443 mm | 5.11 mm | 0.288 mrad |

Both seed-0 runs acquired lock immediately because sufficient geometry was visible at the initial
pose. Production association accepted 80.6% of emitted detections over the full path. This scenario
shows that a correct production-association basin can remain globally accurate over repeated,
field-spanning motion. It does not offset the 6/10 symmetry failures on the six-DoF loop.

### Stationary behavior

With default noise, stationary seed-0 runs ended near but not at truth:

| Mode | Translation RMS | Maximum translation | Final translation | Final rotation |
|---|---:|---:|---:|---:|
| Known correspondences | 0.282 m | 0.780 m | 70.8 mm | 7.30 mrad |
| Production association | 0.282 m | 0.780 m | 72.1 mm | 7.41 mrad |

The high RMS and maximum are dominated by initialization before convergence. The test suite also
contains an exact stationary test and passes its near-truth assertion. For application-level
quality gates, post-lock or fixed warm-up metrics would be more informative than all-sample RMS.

## Feature-association results

### Ground-truth correspondence audit

Production runs were paired with known-correspondence runs using identical scenario, seed, pixel
noise, dropout, and VO settings. Independent sensor streams are deterministic, so corresponding
landmark frames contain identical detections. Each production association was matched by detection
pixel to its known field point.

| Condition | Emitted | Accepted | Acceptance coverage | Correct accepted | Wrong accepted | Accepted precision |
|---|---:|---:|---:|---:|---:|---:|
| Default, 10 seeds | 6,366 | 6,284 | 98.71% | 2,754 | 3,530 | 43.83% |
| 3 px noise + 50% dropout, 10 seeds | 3,184 | 1,098 | 34.48% | 960 | 138 | 87.43% |

No accepted association was unmatchable to a source detection in either audit.

The default precision is dominated by whole-field symmetry: once the wrong global hypothesis is
chosen, many internally consistent associations map detections to the opposite semantic landmarks.
The associator is therefore not merely accepting isolated bad points; it can produce a globally
self-consistent but physically wrong assignment set.

The stressed run appears paradoxically more precise among accepted associations. Heavy noise and
dropout reduce acceptance from 98.7% to 34.5%, making the associator more selective. That precision
does not imply better localization:

- Only 9/10 runs acquired lock.
- One run produced no live pose.
- Lock time ranged from 0.4 to 2.6 seconds.
- Among runs with live output, translation RMS ranged from 0.159 to 5.067 m.
- Final translation error ranged from 66.5 mm to 6.180 m.
- `MaxIterations` occurred at 92.9% of solve checkpoints.

### Why association is the dominant simulated risk

Known and production modes differ only in how emitted semantic pixels are converted to field-point
correspondences. Their synthetic VO and IMU streams are otherwise equivalent for a given seed.
Known mode is consistently millimetric; production mode is seed-dependent and often mirrored.
This is strong controlled evidence that association hypothesis selection, including field symmetry,
is the dominant source of catastrophic error in the tested simulator flow.

### Lock semantics

Global lock currently means that the pipeline accepted a global visual hypothesis and backend
state became available. It does not mean that the hypothesis agrees with ground truth, nor does it
encode ambiguity between symmetric hypotheses.

Any downstream behavior that treats `Locked` as "globally correct" is exposed to confident,
multi-meter errors. A safer design would preserve and expose ambiguity, require temporal evidence
that distinguishes hypotheses, or validate a lock against independent information before promoting
it as trustworthy.

## Visual-odometry results

### What is measured

The simulator generates the exact current-camera-to-previous-camera transform, left-multiplies a
small SE(3) perturbation, and feeds both the frame delta and cumulative odometer to production
localization. Measurements cover:

- VO timestamp and transform-direction handling.
- VO factor ingestion.
- Interaction between VO and semantic reprojection factors.
- Backend correction of accumulated relative motion.
- Live propagation from the latest backend pose using the cumulative visual odometer.
- Recovery after synthetic VO noise, bias, and a one-shot outlier.

### Default VO noise

At 1 mm translation sigma and 1 mrad rotation sigma independently on every 20 ms transition, known
correspondences produced 7.61 mm translation RMS and 2.29 mrad rotation RMS. The estimate therefore
remained far below naive uncorrected random-walk growth due to regular absolute visual updates.

### Increased independent noise

Increasing per-step VO noise tenfold to 10 mm and 10 mrad caused `MaxIterations` on 205 of 210
solve checkpoints, or 97.62%. Production aggregate pose metrics remain dominated by the same field
symmetry split and cannot isolate VO degradation cleanly:

| Metric | Mean | Best run | Worst run |
|---|---:|---:|---:|
| Live translation RMS | 2.897 m | 40.4 mm | 4.809 m |
| Live rotation RMS | 1.884 rad | 17.9 mrad | 3.127 rad |
| Final translation | 3.614 m | 13.4 mm | 6.029 m |
| Maximum translation | 3.648 m | 92.4 mm | 6.029 m |

The useful finding from this row is solver stress, not the mixed-basin pose mean. A follow-up
known-correspondence sweep should be used before assigning a clean VO noise tolerance threshold.

### Persistent bias

The tested per-step bias was 0.5 mm on one translation axis and 0.5 mrad yaw. Production results
were almost unchanged from default because global symmetry dominates the batch:

- Mean final translation: 3.604 m.
- Best final translation: 2.56 mm.
- Worst final translation: 6.006 m.
- `MaxIterations`: 63.3%, close to the 61.9% default-production rate.

Correct-basin runs remained millimetric, indicating that 10 Hz absolute landmark observations can
correct this small systematic drift in the synthetic model. Larger biases and intervals without
visible landmarks were not tested.

### Catastrophic one-shot outlier

At transition 50, from t = 1.00 s to t = 1.02 s, the simulator injected 5 m translation and 170
degrees rotation into one VO delta. Known correspondences isolate recovery from association errors.

| Known-correspondence fault metric across 10 seeds | Result |
|---|---:|
| Runs acquiring lock | 10/10 |
| Mean translation RMS | 1.353 m |
| Mean maximum translation error | 4.907 m |
| Maximum translation range | 4.902-4.910 m |
| Mean rotation RMS | 5.76 mrad |
| Stable recovery below 0.1 m | 180 ms after fault, 10/10 |
| Mean final translation error | 3.00 mm |
| Mean final rotation error | 0.104 mrad |

The translation and rotation perturbations behave differently in global pose error because the
backend's absolute constraints and transform composition rapidly correct orientation while the
cumulative odometer carries a large positional discontinuity until the next backend correction.

The recovery interval closely matches the 200 ms solve cadence. This suggests recovery is provided
by the next absolute backend update, not by explicit VO outlier rejection. The live estimate remains
vulnerable during that interval.

Production-association fault runs again mix correct and symmetric basins. They reached a mean
maximum translation error of 6.42 m, with a 4.90-7.44 m range. Those numbers combine the VO fault
and global association failure and should not be used as a pure VO statistic.

### Real VO front-end robustness mechanisms

Source inspection shows useful defenses in the real stereo VO path, though this simulator does not
exercise or quantify them:

- Invalid and out-of-bounds model outputs are filtered before use.
- Stereo matches require positive disparity and at most 3 px vertical disparity by default.
- At least eight PnP correspondences are required.
- An all-correspondence EPnP estimate is accepted only below the 6 px reprojection threshold.
- Otherwise, PnP RANSAC uses up to 100 iterations at 0.99 confidence.
- Inlier refit is accepted only if left-image RMSE does not regress by more than 10%.
- LM uses a Huber threshold and weighted stereo evidence.
- Right-image validation rejects refinements only when both right RMSE and the bad-observation
  fraction are excessive.
- Failed frame processing resets tracking; failed odometry resets the odometer epoch in the node.

These are reasonable structural safeguards. Their empirical effectiveness requires real or
rendered image sequences with known motion.

## Solver health

The optimizer's iteration-limit rate is the second major concern after association correctness.

| Experiment | `MaxIterations` | Solve checkpoints | Rate |
|---|---:|---:|---:|
| Default known correspondences | 132 | 210 | 62.86% |
| Default production association | 130 | 210 | 61.90% |
| Association stress | 195 | 210 | 92.86% |
| VO noise stress | 205 | 210 | 97.62% |
| VO bias | 133 | 210 | 63.33% |
| VO fault, known | 50 | 160 | 31.25% |
| Figure-eight, known | 86 | 121 | 71.07% |
| Figure-eight, production | 89 | 121 | 73.55% |

Diagnostics are sampled only at the 200 ms solve checkpoints in this table. The report retains the
latest diagnostics on intervening 20 ms samples, so counting every sample would overcount each
solve by approximately ten; this analysis deliberately avoids that error.

`MaxIterations` does not necessarily mean divergence. Several accurate known-correspondence runs
end with tiny residual pose error despite the status. It does mean the configured termination
criteria are usually not reached. Consequences may include:

- Unpredictable solve latency at the configured maximum.
- Reduced runtime margin on robot hardware.
- Sensitivity to larger graphs or worse initial states.
- Inability to distinguish "usable estimate at budget" from "still moving materially at budget"
  using status alone.

The report currently exposes total error and factor residual summaries but not optimization wall
time, iteration count actually used, gradient norm, or final step norm. Those should be measured
before deciding whether the status is benign.

## Performance results

### End-to-end headless throughput

The optimized simulator binary was run sequentially and wrote compact JSON reports.

| Scenario | Simulated duration | Repetitions | Total wall time | Mean wall time | Simulated / wall-time ratio |
|---|---:|---:|---:|---:|---:|
| Six-DoF loop, production association | 4 s | 20 | 8.882 s | 444 ms | 9.01x |
| Field figure-eight twice, production association | 24 s | 5 | 10.467 s | 2.093 s | 11.46x |

The 4-second batch consumed 25.823 s user CPU and 3.512 s system CPU over 20 runs. The 24-second
batch consumed 62.269 s user CPU and 14.018 s system CPU over five runs. Wall time is the relevant
number for simulator throughput; CPU time indicates substantial work and possible internal
parallelism or process-level runtime activity.

The longer scenario amortizes process startup and fixed report overhead, which explains its better
real-time multiplier. Warning output for optimizer iteration limits was enabled and is included in
the timing, making these conservative simulator-throughput numbers for the tested host.

### What the timing does not prove

The performance result cannot be translated into robot frame rate. The dominant real VO work is
absent:

- NV12 image capture and transport.
- XFeat/LightGlue ONNX inference for two current images and previous feature state.
- Device synchronization and output extraction.
- Stereo match triangulation over model outputs.
- Temporal correspondence construction from real matches.
- PnP/RANSAC/LM with real inlier distributions.
- ROS/Zenoh callback scheduling and contention.

The repository contains a KITTI benchmark that separately reports preparation time,
`visual_odometry_ms` average/median/p95/p99, success rate, rotation error, translation error,
relative translation error, scale ratio, correspondence counts, RANSAC behavior, and LM behavior.
No KITTI archives were present in this workspace, so that benchmark was not run. It is the correct
next tool for real VO speed and feature-matching quality.

## Robustness and repeatability

### Determinism

Two independent default-production seed-0 executions produced byte-identical compact JSON:

```text
a0f8343aa7d5c5384b38808fee958824edc4e15a9ec90ebb0c6ea1a9ef84b0da
```

The complete simulator test suite also includes same-seed replay checks and independent sensor RNG
checks. Determinism is a strong property of this harness and makes failures reproducible.

### Automated checks

`cargo test -p localization_simulator` passed:

- 19 library tests.
- 2 binary/CLI tests.
- 0 failures.

Coverage includes VO delta direction and cumulative transform consistency, exact projection bounds,
same-seed sensor replay, one-shot outlier placement, six-DoF interpolation, field-spanning motion,
production association lock, noisy production correspondence checks, complete replay determinism,
report completeness, and JSON serialization.

Passing tests establish implementation consistency with encoded expectations. They do not invalidate
the observed production-association symmetry failure because the existing production lock test
checks that lock occurs, not that the selected global field hypothesis is correct.

### Heavy association degradation

At 3 px landmark noise and 50% independent dropout:

- 49.98% of ideally visible landmarks were emitted, as expected from configured dropout.
- 34.48% of emitted detections were associated.
- Accepted correspondence precision was 87.43% against truth.
- 9/10 runs acquired lock and produced live output.
- Lock latency varied from 0.4 to 2.6 seconds.
- One run never produced live output during the 4-second scenario.
- A successful lock could still be the symmetric field solution.

The system degrades partly by abstaining, which is desirable, but not reliably enough to avoid all
false global hypotheses.

### Failure containment

The one-shot VO test demonstrates recovery but not containment. The production association test
demonstrates a more serious containment issue: a wrong global hypothesis is promoted to `Locked`
and remains coherent. Of the two, the association failure is more dangerous because it is both
large and confidently persistent.

## Threats to validity

The following omissions prevent treating this report as a deployment certification:

- No real images.
- No photorealistic rendering.
- No XFeat keypoint confidence distribution.
- No LightGlue false matches or missed matches.
- No repeated textures, spectators, field wear, shadows, glare, or line occlusion.
- No motion blur, rolling shutter, exposure changes, or camera noise.
- No calibration error in intrinsics, stereo baseline, or robot-to-camera extrinsics.
- No timestamp jitter, dropped VO frames, reordered messages, or stale camera matrices.
- No odometer epoch resets during the synthetic experiments.
- No contact dynamics or foot-height factors.
- No robot vibration or walking-induced camera motion beyond smooth interpolated keyframes.
- No asynchronous operating-system scheduling.
- No robot CPU/GPU performance measurement.
- Only ten random seeds for the principal stochastic sweeps.
- Only one built-in geometry for each long, stationary, and fault scenario.
- The VO noise model is independent Gaussian perturbation plus fixed bias; real VO errors are
  correlated, state-dependent, scale-dependent, and often heavy-tailed.
- The one-shot fault is injected as a known transform perturbation, not generated by a plausible
  bad feature-match set.
- Aggregate report RMS includes initialization behavior and should not be confused with steady-state
  post-lock accuracy.

## Reproduction

Build and run the test suite:

```sh
cargo test -p localization_simulator
cargo build --release -p localization_simulator
```

Generate a default production report:

```sh
target/release/localization_simulator headless \
  --scenario six-dof-loop \
  --association production-association \
  --seed 0 \
  --compact \
  --output /tmp/localization-production-0.json
```

Generate its known-correspondence control:

```sh
target/release/localization_simulator headless \
  --scenario six-dof-loop \
  --association known-correspondences \
  --seed 0 \
  --compact \
  --output /tmp/localization-known-0.json
```

The exact, stressed, bias, and fault rows use complete JSON5 `SimulationConfig` files. A fault
configuration representative of this report is:

```json5
{
  seed: 0,
  association_mode: "KnownCorrespondences",
  landmark_pixel_sigma: 0.5,
  landmark_dropout_probability: 0.0,
  vo_translation_sigma_m: 0.001,
  vo_rotation_sigma_rad: 0.001,
  vo_translation_bias_per_step: [0.0, 0.0, 0.0],
  vo_rotation_bias_per_step: [0.0, 0.0, 0.0],
  vo_outlier: {
    transition_index: 50,
    translation: [5.0, 0.0, 0.0],
    rotation_scaled_axis: [0.0, 0.0, 2.9670597],
  },
}
```

For association stress, change `association_mode` to `ProductionAssociation`,
`landmark_pixel_sigma` to `3.0`, `landmark_dropout_probability` to `0.5`, and remove the outlier.
For VO noise stress, use `0.01` for both VO sigmas. For the bias experiment, use
`[0.0005, 0.0, 0.0]` translation bias and `[0.0, 0.0, 0.0005]` rotation bias per step.

## Recommended next actions

1. Add a regression assertion that production association chooses the correct global hypothesis,
   not merely that `GlobalVisualLock::Locked` is reached. Seeds 0, 2, 3, 5, 6, and 7 from the
   default six-DoF loop are immediate deterministic reproducers.
2. Prevent a symmetric hypothesis from becoming an unqualified lock. Preserve competing hypotheses
   or require disambiguating temporal/independent evidence before promotion.
3. Add a VO innovation gate before live odometry applies a delta. The 5 m / 170-degree fault should
   be rejected or quarantined rather than exposed for 180 ms.
4. Investigate optimizer termination. Record solve wall time, iterations, final step norm, and
   gradient norm; then determine whether `MaxIterations` results are converged enough or budget
   limited.
5. Run the existing KITTI VO benchmark with the production ONNX model and archives. Report p50,
   p95, and p99 runtime, transition success rate, trajectory error, match/correspondence yield,
   RANSAC inliers, and LM acceptance.
6. Add image-level corruption sweeps for blur, brightness, occlusion, repeated texture, stereo
   vertical misalignment, and temporal frame drops. The localization simulator cannot answer these
   questions with direct SE(3) noise.
7. Repeat the simulator matrix with known correspondences for each VO noise and bias level so VO
   sensitivity is not confounded by global association basin selection.
8. Define explicit acceptance criteria before further tuning, for example: zero false locks over a
   specified scenario suite, bounded post-lock p95 translation/rotation error, zero uncontained
   multi-meter live jumps, and solve p99 below the 200 ms backend budget on robot hardware.

## Conclusion

The simulator supports a precise diagnosis rather than a blanket "good" or "bad" verdict.
Downstream of correct correspondences, 3D localization and live visual-odometry propagation are
highly accurate on the tested synthetic trajectories, run much faster than real time on the test
host, deterministically recover from a huge one-shot VO fault at the next backend correction, and
finish with millimetric error.

Production semantic feature association is the limiting factor. It can quickly and confidently
select the field's 180-degree symmetric pose, and the current global lock state does not expose that
the result is wrong. The pipeline should therefore be described as accurate in the correct
association basin, computationally fast in headless estimation-only simulation, recoverable but not
outlier-rejecting for VO faults, and not yet robust against global field symmetry.
