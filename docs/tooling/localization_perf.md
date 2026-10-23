# Localization perf profile — robot 43, 2026-09-27

**Latest iteration:** [100 ms preintegration results](localization_preintegration.md)
compare against the final batched baseline below: 80.6% / 80.1% lower CPU time on
stationary / moving tracking replays. The profiles and raw/averaged-IMU implementation
descriptions in this document are historical stages of that comparison.

## Main findings

The current LM + dense Cholesky + online IMU-bias estimator spends most CPU work evaluating
factor Jacobians and accumulating normal equations. The damped linear solve itself accounts
for only about **0.5% of sampled user cycles**. Two redundant passes are particularly expensive:
the initial-cost probe (~19%) and rebuilding the model for covariance (~17% inside the 18%
covariance phase).

Live-system contention is a separate large cost. In the stationary profile, the replay spent
**66.45 of 110.29 seconds off CPU after preemption**. Optimizing CPU work alone does not remove
that scheduling delay.

## Setup

- Target: `10.1.24.43`, Jetson Orin NX, `5.15.148-rt-tegra`; robot application kept running.
- Boot configuration includes `isolcpus=4-5`; normal scheduler load balancing has fewer cores
  available than the six-core hardware count suggests. All sampled stationary cycles ran on
  cores 0–3 (2.6%, 28.1%, 35.0%, 34.3% respectively). CPU affinity/power settings were not changed.
- Code: current online-bias implementation, fagra `7b37df61`, existing parameters.
- Release optimization, debug information enabled and frame pointers forced for useful call chains.
- Ubuntu's `linux-tools-5.15.0-194` package was downloaded/extracted into `/tmp/localization-perf/tools`,
  rather than installed globally. Its perf version reports `5.15.209`; hardware counters worked.
- Recorded `cycles:u` with frame-pointer call chains and process context-switch events:
  199 Hz for the stationary recording, 499 Hz for synthetic tracking.
- 9,570 stationary samples and 5,533 tracking samples; perf reported **zero lost samples** in both.
- Percentages below are weighted by each sample's cycle period, matching perf's weighted reports,
  not raw sample counts. They are statistical estimates, not exact per-function stopwatch timings.

Workloads:

1. First 30 seconds of the confirmed-stationary robot-43 recording, `learn-with-vo`, 565 cycles.
   Includes raw IMU, sole kinematics, VO, and calibration; no global landmark/recovery workload.
2. Six-second synthetic moving/turning tracking benchmark, 121 cycles, global field tracking,
   500 Hz IMU, 50 Hz VO, 10 Hz known correspondences; no foot factors.

The first run's estimates and learned biases matched the earlier release replay. Synthetic
tracking again produced 119/121 field estimates with maximum error 9.63 mm / 0.241°.

## CPU time versus wall time

A separate `perf stat` stationary run measured:

| Metric | Measurement |
| --- | ---: |
| Total process wall time | 108.061 s |
| Scheduled task-clock | 43.886 s |
| CPU utilization | 0.406 cores |
| User / system time | 42.536 / 1.445 s |
| Context switches | 197,955 |
| CPU migrations | 80 |
| Instructions / cycle | 3.26 |
| Estimator wall time, excluding decoding/setup | 107.466 s |
| Median / p95 estimator cycle | 178.4 / 312.8 ms |

The call-chain recording was a different run. Pairing its process `PERF_RECORD_SWITCH OUT/IN`
events gives this approximately complete single-thread time accounting:

| Recorded interval | Time | Wall-time share |
| --- | ---: | ---: |
| Scheduled/on CPU (remainder) | 43.718 s | 39.6% |
| Off CPU after preempted switch-out | 66.448 s | 60.2% |
| Off CPU after voluntary/blocked switch-out | 0.127 s | 0.1% |
| Total observed interval | 110.292 s | 100% |

There were 216,923 paired preempted switch-outs and 246 voluntary/blocked pairs. The recorded
estimation loop itself took 109.565 s, median 181.2 ms / p95 322.6 ms. MCAP decoding/startup
accounted for only 0.43% of sampled cycles; blocking I/O does not explain the large wall-time gap.

The synthetic tracking profile took 18.770 s of estimation wall time, median 153.2 ms /
p95 295.3 ms. Its test worker had 11.008 s off CPU after preemption. The test-harness parent
waited for the worker; that overlapping wait must not be added to worker elapsed time.

## Non-overlapping CPU phase breakdown

| Phase | Stationary replay | Moving tracking |
| --- | ---: | ---: |
| Actual LM optimization call | **60.61%** | **64.21%** |
| Extra initial-cost evaluation call | **19.04%** | **17.89%** |
| Separate covariance query | **18.13%** | **16.63%** |
| Window marginalization/retirement | 1.45% | 0.98% |
| Other estimation work | 0.16% | 0.19% |
| Input ingestion | 0.18% | 0.10% |
| Other / decoding / startup | 0.43% | <0.01% |

Call-site attribution distinguishes the two calls to the same LM method. `addr2line` mapped
stationary `Estimator::solve_graph+0x490` to `estimator/mod.rs:407`, the `evaluation` options
call, and `+0x4c8` to line 409, the real optimizer call. The tracking executable's respective
offsets are `+0x48c` and `+0x4c4`, mapping to the same lines. The initial-cost probe sets gradient
tolerance to `f64::MAX`, but fagra still builds all Jacobians and normal equations before
checking that tolerance.

## What the CPU phases contain

These are alternative views of the same samples, **not additional costs to sum with the table above**.

### Inclusive factor linearization and assembly

| Factor family | Stationary replay | Moving tracking |
| --- | ---: | ---: |
| IMU gyro + acceleration | 43.75% | 49.92% |
| SDK tilt priors | 23.29% | 27.47% |
| Foot nonpenetration | 15.86% | — |
| Visual odometry | 3.28% | 6.78% |
| Other linearization work | 2.10% | 4.81% |
| **All normal-model linearization** | **88.28%** | **88.98%** |

### Selected low-level hotspots

| Operation | Stationary replay | Moving tracking |
| --- | ---: | ---: |
| Dense normal-equation accumulation kernels, self time | **34.78%** | **35.35%** |
| `memcpy`, self time | **10.42%** | **10.43%** |
| `CheckedSink::residual`, 3-row validation/dispatch, self | 5.34% | 5.50% |
| Pose-Jacobian finiteness scan, self | 5.29% | 3.90% |
| `LinearizedPoseSpline::pose_from_increments`, self | 5.18% | 4.06% |
| `LinearizedPoseSpline::kinematics_from_increments`, self | 4.93% | 4.77% |
| **Dense LM `solve_damped`, inclusive** | **0.46%** | **0.54%** |

Copies occur predominantly underneath factor visitation and spline/Jacobian evaluation, rather
than estimator rollback snapshots. Frame-pointer unwinding through libc assembly can omit the
immediate caller, so the profile supports the broad ownership of these copies, not an exact
source-level list of copy expressions.

## Prioritized optimizations

1. **Remove the initial-cost Jacobian pass.** `estimator/mod.rs` calls `solve_batch` solely to
   obtain `initial_cost`. Obtain that cost from solver reporting/statistics or a cost-only API.
   Preserve diagnostic information and rejection semantics; do not substitute a different objective.
   This redundant call owns ~19% of current CPU work.
2. **Reuse the final undamped model for covariance.** `estimator/covariance.rs` calls
   `graph.joint_covariance`, rebuilding the model. In the stationary profile, **16.73% of all
   cycles** are linearization inside this covariance query. Fagra already exposes
   `LevenbergMarquardt::solve_batch_with_covariance`; its workspace skips relinearization when
   the final model is current. Keep a correct fallback for accepted nonconverged motion results.
3. **Batch/share high-rate IMU and tilt preparation.** Tilt is currently a standalone factor per
   sample, alongside batched gyro observations. Reuse spline preparation and matching-time
   evaluations; consider rotation-only evaluation for residuals that do not depend on translation.
   This preserves measurements instead of arbitrarily dropping samples or changing weights.
4. **Avoid work for exactly inactive constraints and zero Jacobians.** Both inactive foot rows
   have zero cost/Jacobians, but currently still incur full pose Jacobian evaluation and assembly.
   Zero gyro roots on acceleration-only observations also emit zero rows. An inactive fast path
   must retain validation and re-evaluate activity on every new linearization/trial.
5. **Improve dense accumulation's handling of sparse blocks/columns.** Gyro/tilt Jacobians contain
   many exactly-zero columns, and bias blocks add more. The generic tiny-row loop still multiplies
   them. Benchmark an exact-zero fast path rather than changing the underlying solver again.

The first two items identify roughly one-third of current CPU work as duplicate model-building
opportunities, but that is **not a measured speedup**. The Cholesky factorization is already cheap.
Scheduling contention remains a separate constraint; changes to CPU allocation must account for
the robot's deliberately isolated control cores.

## Artifacts and reproduction

Robot directory: `/tmp/localization-perf/`.

- `stationary.data`, `tracking.data`: raw perf recordings (also copied to `/tmp/opencode/` on host).
- `replay`, `tests`: symbolized executables.
- `stationary.stat`, `stationary.stat-run.jsonl`, `stationary.record-run.jsonl`.
- `tracking.record-run.txt`, `tracking.record-log.txt`.
- `stationary.self.txt`, `stationary.children.txt`, `*.stacks.txt` and offset-bearing call chains.

Host interactive flamegraphs, with period-weighted cycle widths:

- `/tmp/opencode/localization-stationary.svg`
- `/tmp/opencode/localization-tracking.svg`

The host analysis script `/tmp/opencode/profile_summary.py` groups call chains and pairs switch
events per thread; folded stacks and summaries are alongside it. Raw perf reports were used to
cross-check weighted totals and leaf hotspots. Debug-build SHA-256:

- `replay`: `55c9547b8c93968ebbc8050c9793c89e303dc6ad258444eccb66396d8d0afe22`
- `tests`: `4dc4ac51204c14b5345bcc1ebc896100b8cb3c73a83a0d97f866c2a0555f6b20`

Profiling config (`target/localization-perf.toml`, ignored build artifact):

```toml
[profile.release]
debug = 1
strip = "none"

[target.aarch64-unknown-linux-gnu]
rustflags = ["-C", "force-frame-pointers=yes"]
```

```sh
target/debug/pepsi build crates/nodes/localization-3d --env podman --release \
  --example imu_calibration_replay --tests --config /hulk/target/localization-perf.toml

# On robot 43, after copying the profiling executables:
PERF=/tmp/localization-perf/tools/usr/lib/linux-tools-5.15.0-194/perf
sudo "$PERF" stat -e task-clock,cycles,instructions,context-switches,cpu-migrations,page-faults \
  -- /tmp/localization-perf/replay \
  /home/booster/hulk/logs/2026-09-27T17:24:08.443+08:00/recording.mcap 30 learn-with-vo
sudo "$PERF" record -o /tmp/localization-perf/stationary.data \
  -e cycles:u -F 199 --call-graph fp --switch-events --sample-cpu \
  -- /tmp/localization-perf/replay \
  /home/booster/hulk/logs/2026-09-27T17:24:08.443+08:00/recording.mcap 30 learn-with-vo
```

For tracking, replace the command with `/tmp/localization-perf/tests benchmark_tracking_estimation
--ignored --nocapture --test-threads=1`, use a separate output file and 499 Hz. Report ownership
should match the user running `perf report`; the archived `.data` files were made readable by booster.

These are profiles of the isolated current estimator under live robot load, not a profile of
all robot services or of a prolonged LostTrack/recovery episode. No production solver changes
were made during profiling.

## Implemented improvements and matched measurements

The profiling recommendations were subsequently implemented in localization:

- Removed both evaluation-only optimizer calls. Successful reports supply initial/final cost;
  LM statistics supply the accepted final cost on `NoProgress`/`NoConvergence`. LM's strictly
  cost-decreasing acceptance remains in force. The optional initial-cost diagnostic is unknown
  when a report is unavailable after accepted steps, rather than paying for another model build.
- Use `solve_batch_with_covariance` and its cached undamped model on convergence. Accepted
  partial motion results still use fresh covariance. Whole-update rollback, including IMU bias,
  and visual/heading/tilt acceptance checks remain intact.
- Fold the SDK tilt residual into each gyro observation's batch, reusing spline preparation,
  rotational exponentials, and their Jacobians. Noise roots and observations are unchanged.
- Skip zero gyro rows on acceleration-only observations. Omit exactly-zero Jacobian blocks
  without allocating or losing constant residual cost; nonfinite blocks still reach validation.
- Inactive feet use value-only pose evaluation and an empty visited factor scope. Activity and
  validity are checked again on every linearization/trial; active constraints still emit derivatives.

Fagra remains the upstream dense solver. Partial-column sparsity inside nonzero blocks is still
handled by its generic accumulation kernel; no local replacement backend was introduced.

### Protocol

On robot 43 with the live application running, used the preserved `replay`/`tests` executables
as baseline and built `replay-after`/`tests-after` with the **same release, debug, and frame-pointer
settings**. Three runs of each workload/variant, interleaved before→after, after→before,
before→after. `perf stat` measured task-clock, cycles, instructions, context switches, and migrations.
Only one benchmark ran at a time. No CPU affinity, power, sensor, or solver-limit changes.

Table entries are medians across three runs (percentile columns are medians of per-run percentiles):

| Workload | Version | Estimator wall time | Process CPU time | Median cycle | p95 cycle |
| --- | --- | ---: | ---: | ---: | ---: |
| 30 s stationary recording, 565 cycles | Before | 87.241 s | 41.564 s | 152.9 ms | 221.5 ms |
| Same | After | **43.014 s** | **20.144 s** | **75.2 ms** | **116.3 ms** |
| 6 s moving tracking, 121 cycles | Before | 15.566 s | 7.424 s | 133.5 ms | 244.1 ms |
| Same | After | **9.403 s** | **4.489 s** | **73.1 ms** | **153.2 ms** |

- Stationary: **2.03× wall-time speedup, 2.06× CPU-time speedup** (51.5% less CPU time).
  Instructions fell from 263.09 billion to 121.31 billion (53.9% reduction).
- Moving tracking: **1.66× wall-time speedup, 1.65× CPU-time speedup** (39.5% less CPU time).
  Instructions fell from 46.57 billion to 27.27 billion (41.4% reduction).
- CPU counters include process initialization/recording decoding; estimator wall time excludes them.
  Live-system scheduling explains much of the remaining gap. These workloads still exceed a 50 ms
  cycle budget under this load; this change does not establish a hard real-time guarantee.

Raw total estimator wall times / CPU seconds, in run order:

| Workload/version | Wall seconds | CPU seconds |
| --- | --- | --- |
| Stationary before | 86.361, 90.225, 87.241 | 41.365, 41.616, 41.564 |
| Stationary after | 41.006, 44.258, 43.014 | 20.098, 20.144, 20.229 |
| Tracking before | 15.566, 16.687, 14.297 | 7.424, 7.429, 7.351 |
| Tracking after | 9.403, 9.065, 9.911 | 4.473, 4.489, 4.524 |

### Correctness and follow-up profile

- Stationary replay: identical 564/565 estimates and 565 gradient-converged solves (one initial
  covariance failure). Final position differs by less than 2e-13 m per component; learned
  accelerometer bias differs by less than 7e-16 m/s²; bias covariance differs by at most 1.1e-13.
- Moving tracking: identical 119/121 field estimates, one failed solve, maximum error
  9.634 mm / 0.240922° at the printed precision.
- The older divergence replay retains 330/333 local estimates, zero field estimates and
  maximum absolute height 0.605 m.
- All 27 localization tests passed on robot 43. Host factor/Jacobian and simulator regressions,
  allocation checks, cached-versus-fresh covariance equivalence, partial-solve handling, and the
  fused-versus-separate IMU/tilt objective check passed.

A follow-up stationary perf recording captured 4,337 samples. The separate covariance rebuild and
initial-cost probe are absent. Foot linearization/assembly drops from 15.86% of the old CPU profile
to 1.51% of the smaller new profile. Dense accumulation and spline/Jacobian copies remain the
largest remaining hotspots (~33.8% and ~12.7% self time). The generic analysis script labels the
combined LM/cached-covariance call as “LM incl. evaluation-only probes”; that historical label
does not indicate probes in the new implementation. This run covered 40.36 s wall time, including
20.43 s off CPU after preemption.

Matched-run CSV counters and outputs are under `/tmp/localization-perf/` on robot 43 and
`/tmp/opencode/localization-improvements/` on the host, named `{stationary,tracking}-{before,after}-{1,2,3}`.
The host directory also contains `summarize.py`, the new call-chain summary, and `stationary-after.svg`.
The follow-up perf data is `/tmp/localization-perf/stationary-after.data` on the robot.

SHA-256 of measured optimized executables:

- `replay-after`: `d5320fd483dc995e25b405526019f08c86f5238d7cac5ad97e7d332bce1b492d`
- `tests-after`: `424714b88f286cf58b3ca786a6dc82f1ac50d4f79293766e2c27418ad94cc8b5`

## Repeated profile of the current optimized implementation

Repeated both profiles on robot 43 after the optimization comparison. Executable hashes match
`replay-after` and `tests-after` above. The application remained active (~203% process CPU,
load average ~13–14); `isolcpus=4-5` was unchanged. All sampled execution used cores 0–3.
Stationary recording sampled at 499 Hz (13,665 samples), moving tracking at 999 Hz (6,306 samples).
Both perf reports show **zero lost samples**. Percentages are period-weighted sampled user cycles.

### Current elapsed time and CPU usage

Separate, nonsampling `perf stat` runs measured:

| Metric | Stationary recording (565 cycles) | Moving tracking (121 cycles) |
| --- | ---: | ---: |
| Estimator wall time | 42.312 s | 10.270 s |
| Total process wall time | 42.998 s | 10.286 s |
| Scheduled task-clock | 20.312 s | 4.641 s |
| User / system CPU time | 19.540 / 0.725 s | 4.368 / 0.190 s |
| Average cores utilized | 0.472 | 0.451 |
| Median / p95 estimator cycle | 74.4 / 116.8 ms | 84.2 / 171.3 ms |
| Retired instructions | 121.44 billion | 27.35 billion |
| Context switches | 119,069 | 27,467 |

The separate sampling runs measured estimator wall times of 44.214 s and 10.153 s.
Context-switch pairing in those runs showed:

- Stationary thread: 44.752 s observed span, 24.583 s preempted off CPU (54.9%),
  0.010 s voluntary/blocked, and approximately 20.159 s scheduled.
- Tracking worker: 10.155 s observed span, 5.628 s preempted off CPU (55.4%), and
  approximately 4.528 s scheduled. The harness parent waited for the worker and is excluded
  from this accounting. Do not sum overlapping parent/worker waits.

These runs confirm that scheduling still approximately doubles elapsed time relative to CPU work.
Differences from earlier wall-time runs reflect changing live load; the instruction counts remain stable.

### Non-overlapping CPU phase shares now

| Phase | Stationary | Moving tracking |
| --- | ---: | ---: |
| Factor linearization and normal-equation assembly | **86.81%** | **89.87%** |
| Nonlinear cost evaluation | 5.57% | 5.00% |
| Marginalization / window retirement | 2.37% | 1.36% |
| Layout / dependency preparation | 1.93% | 1.18% |
| Damped linear solve | **0.97%** | **1.00%** |
| Cached covariance extraction | **0.65%** | **0.86%** |
| Input ingestion | 0.29% | 0.16% |
| Other estimation | 0.46% | 0.52% |
| Decoding / setup / other | 0.95% | 0.04% |

There are no samples in the old extra initial-cost solve or separate fresh covariance rebuild.
Unlike the historical analysis script, this run classifies `Workspace::covariance` and
`SelectedCovariance::compute` explicitly instead of folding cached covariance into the combined
LM entry point. Shared compiler-generated helper names are attributed by their real parent stacks.

### Dominant self-time functions

These are portions of the phase table, not additional costs:

| Function / operation | Stationary | Moving tracking |
| --- | ---: | ---: |
| `DenseNormalCholesky::accumulate`, three-row residuals | **33.62%** | **33.69%** |
| `memcpy` (mostly spline/Jacobian temporaries) | **13.19%** | **12.84%** |
| `LinearizedPoseSpline::kinematics_from_increments` | 6.24% | 5.74% |
| `CheckedSink::residual`, three-row validation/dispatch | 5.49% | 5.00% |
| `LinearizedPoseSpline::pose_from_increments` | 4.31% | 4.45% |
| Finiteness scan over pose Jacobians | 3.79% | 4.34% |
| `PoseSpline::pose_from_increments` (value evaluation) | 2.61% | 1.30% |

By factor family, including each family's derivative evaluation and matrix assembly:

| Family | Stationary | Moving tracking |
| --- | ---: | ---: |
| Combined gyro, tilt, and accelerometer | **79.07%** | **78.12%** |
| VO | 3.96% | 6.88% |
| Feet | 1.32% | — |
| Other linearization | 2.46% | 4.88% |

The remaining priorities are dense accumulation of partially sparse Jacobian blocks, reducing
large spline/Jacobian value copies, and avoiding full pose/linear-acceleration derivatives for
rotation-only residuals. Input ingestion, covariance extraction, marginalization, and Cholesky
factorization are no longer the principal CPU targets. Validation checks should be consolidated
only where their guarantees can be preserved, not simply removed.

Calibration and output checks still match the optimized baseline: stationary replay produces
564/565 estimates with identical printed final pose; tracking produces 119/121 field estimates,
maximum errors 9.634 mm / 0.240922°.

Artifacts on robot 43: `/tmp/localization-perf/current-{stationary,tracking}.*`.
Copies, phase-analysis JSON, and the analysis script are on the host in
`/tmp/opencode/localization-current-perf/`. Updated interactive flamegraphs:

- `/tmp/opencode/localization-current-perf/current-stationary.svg`
- `/tmp/opencode/localization-current-perf/current-tracking.svg`

This repeat run made no production-code or tuning changes. Statistical samples locate CPU work;
the task-clock and switch-event measurements separately account for scheduling delay.

## Local refinements: complexity versus measured gain

Implemented and benchmarked the subsequent review's local changes in two stages, with no fagra
dependency/kernel changes and no new selective-derivative API:

1. **Small:** `each_ref()` when mapping borrowed tilt Jacobians, plus propagation of only the
   already-active angular derivative prefix (0, 2, 3 matrices instead of 4 at each recurrence stage).
2. **Batched:** the small changes plus acceleration batches keyed by pose segment, following the
   existing gyro batch pattern. Moved the acceleration information root from the batch model to
   each observation; every mean retains its actual averaging-duration/noise weight. Retirement
   uses the segment map, and recovery continues through the same insertion path.

The baseline already contains the earlier redundant-pass removal and fused gyro/tilt changes.
All three binaries used the same release/debug/frame-pointer build settings. Three rounds rotated
execution order: baseline→small→batched, small→batched→baseline, batched→baseline→small, with one
benchmark at a time on robot 43 while the robot application remained active. `perf stat` collected
task-clock and instructions independently of estimator wall-time measurements.

Medians across three runs:

| Workload | Variant | Process CPU s | Instructions, billions | Estimator wall s | Median cycle ms | p95 cycle ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 30 s stationary window, 565 cycles | Baseline | 20.118 | 121.332 | 42.987 | 73.94 | 120.64 |
| Same | Small | 19.311 | 119.864 | 35.740 | 62.19 | 113.10 |
| Same | Small + batched | **18.780** | **118.023** | 34.576 | 57.76 | 106.04 |
| 6 s moving tracking, 121 cycles | Baseline | 4.557 | 27.288 | 10.203 | 74.08 | 181.85 |
| Same | Small | 4.413 | 27.075 | 8.361 | 63.49 | 144.09 |
| Same | Small + batched | **4.300** | **26.615** | 8.842 | 64.49 | 142.83 |

Incremental reductions, ratios of the corresponding medians:

| Change | Stationary CPU | Tracking CPU | Stationary instructions | Tracking instructions |
| --- | ---: | ---: | ---: | ---: |
| Small versus baseline | 4.0% | 3.1% | 1.2% | 0.8% |
| Batching versus small | 2.7% | 2.6% | 1.5% | 1.7% |
| All retained changes versus baseline | **6.6%** | **5.6%** | **2.7%** | **2.5%** |

The wall-time variation is much larger than these algorithmic gains. For example, tracking's
small variant ranged from 5.032 to 9.862 seconds, and batched from 5.141 to 10.359 seconds.
Do not interpret the 13–20% median wall-time changes as a reliable code-only speedup. Instruction
counts were stable, and batching reduced CPU time in all three matched rounds for both workloads.

**Decision:** retain both stages. The tiny changes add no API or lifecycle machinery. Batching
delivers a modest but repeatable gain while reusing the existing segment-map pattern and putting
measurement-specific whitening with the measurement. The broader compact/selective spline
refactor remains deferred; this result does not establish its benefit.

Correctness and checks:

- Small changes produced identical printed stationary pose, bias, and covariance results.
- With batching, final stationary position components differ from baseline by less than 8e-14 m,
  bias components by less than 3e-16 m/s², and bias covariance entries by less than 5e-14.
- Every variant returned 564/565 stationary estimates and 565 gradient-converged solves
  (one initial covariance failure). Tracking retained 119/121 field estimates and maximum errors
  of 9.634 mm / 0.240922°.
- All 28 localization tests passed on robot 43, including new grouped-versus-singleton unequal-weight
  cost/information checks and extended acceleration retirement/recovery coverage.
- Host derivative checks cover unequal observation roots in f32/f64; allocation checks, simulator
  regressions, and the schema doctest passed. The old divergence replay retains 330/333 estimates,
  no field estimates, and max |height| 0.605 m.

Raw CPU times in run order:

| Workload | Baseline | Small | Batched |
| --- | --- | --- | --- |
| Stationary | 20.263, 19.776, 20.118 | 19.311, 19.293, 19.761 | 18.906, 18.780, 18.750 |
| Tracking | 4.557, 4.469, 4.563 | 4.413, 4.127, 4.457 | 4.300, 4.038, 4.326 |

Artifacts: robot `/tmp/localization-perf/review-{stationary,tracking}-{base,small,batched}-{1,2,3}.*`;
host copies and `summarize.py` in `/tmp/opencode/localization-review-impact/`. The baseline remains
`replay-after`/`tests-after` above. Additional measured executable hashes:

| Executable | SHA-256 |
| --- | --- |
| `replay-small` | `cab635791caaf70fe3cfd3ca1750c3b36a8a754e04db38a8bde878f7455cb9b4` |
| `tests-small` | `47cf74aa7047a0d4b741bc408785e0e84b50e060f79e9f444d0522b569a67b11` |
| `replay-batched` | `d4e8a3ae3cee7bb5d0bb07ffba2b65ece8c7368f1a97a471f65f8f8469bdd079` |
| `tests-batched` | `f11e24b4b2bbc1dd5f110f0caeb8440450e06d47a73cf00f6f97aef9108dbf81` |
