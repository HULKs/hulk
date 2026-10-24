# Relocalization replay

**Continuity work is experimental.** The 10/10 result below describes the
earlier reinitializing implementation, not the current retained-history prototype.
Current prototypes have not met both the checkpoint and continuity requirements.

## Retained-history experiments

### Recorded checkpoint reprojection reference

Run `cargo run -p localization-3d --example relocalization_replay -- recovered.mcap 425 checkpoint-rmse`.
This diagnostic does not rerun localization or fit a pose to minimize pixel error.
It selects the recorded association frame nearest each checkpoint, interpolates
the recorded field pose to exposure time (translation lerp, rotation slerp), and
uses that frame's recorded camera extrinsics/intrinsics and correspondences.
RMSE is `sqrt(mean(dx² + dy²))` over features, with no robust weighting or
outlier removal. Epoch/generation boundaries and behind-camera points reject
the measurement. These are recomputed reference residuals, not recorded solver
costs or proof that each association was accepted by the original estimator.

| Checkpoint | RMSE px | Features | Exposure minus checkpoint, ms | Pose interpolation bracket, ms |
| --- | ---: | ---: | ---: | ---: |
| 02:42.236 | 4.90 | 5 | -3.5 | 20.1 |
| 02:50.157 | 2.56 | 3 | +92.3 | 50.1 |
| 03:58.487 | 3.71 | 7 | +11.9 | 156.3 |
| 04:06.126 | 3.98 | 7 | -342.8 | 128.3 |
| 04:44.751 | 3.32 | 3 | -1.2 | 42.1 |
| 04:56.848 | 4.16 | 3 | +1.2 | 40.1 |
| 05:47.541 | 3.38 | 4 | -24.8 | 156.2 |
| 06:00.263 | 6.50 | 8 | +20.5 | 52.1 |
| 06:37.793 | 10.01 | 5 | +6.3 | 42.1 |
| 07:02.793 | 8.46 | 3 | -77.1 | 52.1 |

04:06.126 has no recorded association frame within 100 ms; its value is a
nearby-frame reference only. The 03:58 and 05:47 results also use wider than
100 ms interpolation brackets. JSON includes timing flags and per-feature
pixel errors: `/tmp/opencode/checkpoint-reprojection.json`. Competing matches
are not evaluated by this diagnostic. A tight 2–3 px threshold would exclude
most of these user-validated reference poses.

### Correction audit (accuracy and recovery, not jump suppression)

**Current implementation:** the experimental pose-revision gate and its cached
publication checkpoint have been removed, including their config parameter.
The gate-on/off comparisons below describe the preserved audit executable
`/tmp/opencode/relocalization-audit`; current builds no longer need a
`no-revision-gate` argument. Existing pixel, heading, tilt, ambiguity and
convergence checks remain. Production knot spacing is still 200 ms pending
the low-cost accuracy experiments.

After removing the gate, full 100/100 ms runs with both production and precise
solver settings pass 9/10 checkpoints. Both fail only 07:02.793 (1.052/1.030 m
horizontal error, 9.73°/9.47° yaw error), despite Tracking. These runs overlap
cost diagnostics and are not isolated runtime measurements. At image time
422.749 s, both accept two features with about 5 px fitted RMS. An extra global
search on every sparse local success was considered and discarded: local
certification already requires the retained detections, and global matching
uses the same filter. It cannot obtain three retained features where only two
survive that shared filter.

`cost-200`, `cost-100` and `cost-50` enable TRACE-only cost diagnostics before
recovery seeding, after seeding and after optimization. They separately evaluate
active preintegrated IMU, IMU boundary and pending reprojection costs. These are
subsets, not a complete decomposition of marginal priors, odometry, historical
visual factors or constant marginalized costs. At solve time 237.834 s (200 ms),
seeding lowers pending visual cost 245.05->5.53 while raising integrated IMU
cost 13.85->428.45; optimization restores IMU cost to 14.36 while giving back
visual fit (cost 14.71). This demonstrates a motion/visual tradeoff, not faulty
raw IMU measurements or proof that marginal priors are the cause.

`quiet` as the fourth positional argument disables verbose audit logging for
paired runtime measurements. Production grid defaults remain unchanged until
the accuracy/runtime comparison completes.

The clarified objective permits valid corrective jumps. Compare each fitted
correction against later images, rather than treating jump count as failure.
Replay audit logging records the exposure-time field pose and pixel RMS before
optimization, in the trial, and on acceptance. `audit-reference` exports recorded
poses, recorded correspondences and raw VO transformed through endpoint camera
geometry. Normal replay reports also include admitted frames and output poses.

```
cargo run -p localization-3d --example relocalization_replay -- recovered.mcap 425 audit-reference > /tmp/opencode/relocalization-audit-reference.json
# Run each grid with and without the extra revision gate; all other gates remain.
cargo run -p localization-3d --example relocalization_replay -- recovered.mcap 425 grid-50-precise no-revision-gate > /tmp/opencode/replay.json 2> /tmp/opencode/replay.log
python3 crates/nodes/localization-3d/examples/audit_relocalization.py /tmp/opencode/relocalization-audit-reference.json /tmp/opencode/replay.json /tmp/opencode/replay.log
```

The analyzer requires host numpy/scipy, adds no robot dependencies, and runs
assert-based geometry checks. A large correction means >=0.25 m translation
or >=5° rotation at the same exposure time; these are analysis selectors, not
acceptance gates. Rich-frame summaries require >=3 features and generation >0.
Later-image tests freeze each candidate, propagate it using contiguous **raw VO**
and compare with recorded correspondences around 0.15, 0.30 and 0.50 s later.
They never use future optimized poses to propagate the candidate. Missing VO
chains or image gaps leave a case inconclusive. Two later checks both improving
and <=10 px are supporting evidence, not a universal correctness certificate.

Recorded poses outside the ten validated timestamps are comparators, not ground
truth. Original association identities are also not independent manual labels.
Reference interpolation requires the same epoch/generation and <=200 ms bracket.
The audit cannot prove absence of unsampled global matches or the correctness of
a repeated-pattern match. Global association still uses bounded sampled search
and an exact assignment check at the winning pose, not exhaustive pose search.

The 04:39.333 image's accepted global correction changes its exposure pose by
1.052 m and reduces fitted RMS 72.81->6.59 px. Raw-VO predictions for three later
frames improve 69.16->6.70, 65.67->6.06 and 67.08->6.98 px. That strongly supports
the correction, despite increased disagreement with the unvalidated recorded XY
at that instant. Rejecting it solely as a jump would discard useful evidence.

At 05:59.550, a five-feature trial rejected by the revision gate reduces RMS
44.32->2.61 px. All five compared identities match the recorded associations;
two later raw-VO checks improve 44.22->4.78 and 45.36->5.46 px. Recorded yaw
disagreement falls 7.70°->0.08°. This is evidence that the experimental gate
rejects useful corrections. The 06:17.183 correction lacks later raw-VO coverage
and its recorded comparator has 123.51 px RMS on those correspondences: neither
the comparator nor the fitted residual alone establishes correctness there.

#### Completed controlled audit

Same current implementation, full 425 s recording, immediate global fallback.
`no-revision-gate` sets only the experimental revision threshold to 1e12;
pixel, heading, tilt, convergence and other checks remain enabled. No production
acceptance rules were changed for this analysis.

| Grid / preintegration | Revision gate | Checkpoints | LostTrack episodes | Time in LostTrack | Field / Tracking outputs |
| --- | --- | ---: | ---: | ---: | ---: |
| 50 / 50 ms, precise solver | on | 10/10 | 6 | 12.88 s | 9,962 / 9,473 |
| 50 / 50 ms, precise solver | effectively off | 10/10 | 4 | 10.40 s | 9,989 / 9,518 |
| 200 / 100 ms, production solver | on | 6/10 | 8 | 122.89 s | 8,528 / 7,267 |
| 200 / 100 ms, production solver | effectively off | 6/10 | 8 | 94.51 s | 8,530 / 7,762 |

LostTrack durations use lifecycle transitions, including intervals ending at
recording end; they do not include Startup or prove accuracy while Tracking.
With the gate off, the 50 ms losses are 8.914, 0.507, 0.244 and 0.731 s. During
the longest (03:27.700–03:36.614), only two single-feature frames are proposed;
no >=3-feature frame is proposed. This does not prove the raw images lack
information: association may itself have failed to extract/match it.

Source-time coverage by an output no older than 100 ms is also computed, across
424.30 s of replay lifecycle. Field coverage: 78.85%, 79.12%, 66.13%, 66.14% in
table order. Tracking coverage: 76.18%, 76.74%, 54.18%, 59.79%. These are source-
time fresh-output coverage, not delivery-time availability or true accuracy.

Visual rollback reason counts (multiple updates can occur in one solve):

| Grid / gate | Non-convergence | Pixel validation | Revision gate | Heading disagreement |
| --- | ---: | ---: | ---: | ---: |
| 50 ms / on | 32 | 84 | 40 | 0 |
| 50 ms / off | 35 | 87 | 0 | 0 |
| 200 ms / on | 969 | 243 | 85 | 51 |
| 200 ms / off | 959 | 253 | 0 | 35 |

The revision gate demonstrably blocks supported corrections and worsens recovery
availability. At 50 ms, 32 of 46 accepted rich significant corrections with at
least two later checks improve every check to <=10 px; gate-off gives 44 of 54.
Five rejected corrections with gate-on meet the same supporting criterion;
gate-off gives zero. Cases failing that criterion are **not automatically wrong**:
raw VO error, image geometry and correspondence error also affect the checks.

The 200 ms problem remains with the gate off. At 03:57.832, seven proposed global
correspondences agree with the recorded frame. The recorded pose fits them at
3.09 px; the optimized 200 ms trial remains at 16.67 px and is rejected for
non-convergence. Nearby frames show the same pattern (about 3.1 px reference,
17 px trial). Thus there is useful visual evidence near the failed 03:58
checkpoint, but the retained graph/optimization cannot reconcile it. This
isolates the failing layer, not whether spline approximation, marginal-prior
inconsistency or numerical scaling is the underlying cause. Earlier 60-iteration
200 ms experiments also failed; merely increasing iterations is not established
as a fix. The 50/200 comparison changes solver and preintegration too, so it
does not by itself isolate knot spacing.

Gate-off is **not deployment certification**. Near 06:16–06:18 it accepts
successive metre-scale revisions (3.62 m and 4.22 m at image times 376.250 and
377.116 s). The first has 5.44 px fitted RMS and one later check at 8.25 px;
the second has 8.10 px fitted RMS but lacks usable later raw-VO checks. Recorded
poses are unreliable comparators there. Correctness of this field branch still
needs manual landmark identification or independent pose evidence. Neither
revision magnitude nor these low fitted errors settle it.

Conclusions for the clarified objective:

1. Do not use the experimental revision gate as the definition of correct recovery.
   Some large corrections are supported by later observations and should be allowed.
2. Retain immediate fallback as a way to admit information sooner; it is not a
   correctness certificate, and sampled alternatives must still be discriminated.
3. Use 50 ms as the working reference, not a proven final configuration. At 200 ms,
   diagnose factor-wise motion/visual/prior conflict and solver termination in a
   validated interval before changing acceptance rules or weakening convergence.
4. Investigate the longest 50 ms dropout at the association layer: only two sparse
   matches reach localization. A localization-only recovery change cannot consume
   rich observations that association does not publish.
5. Verify the disputed 06:16–06:18 branch against actual landmark identities before
   claiming all accepted corrections are correct. This remains unresolved.

Raw replay reports/logs: `/tmp/opencode/relocalization-audit-{50,50-no-gate,200,200-no-gate}.{json,log}`.
Event-level comparisons: `/tmp/opencode/relocalization-analysis-{50,50-no-gate,200,200-no-gate}.json`.
Recorded inputs: `/tmp/opencode/relocalization-audit-reference.json`. Instrumented
stage times are approximately 267, 265, 141 and 137 s; they include verbose debug
audit logging and some overlap with compilation/analysis, not production benchmarks.

Recovery observations now update the existing graph and its marginal priors;
only initialization creates a new field trajectory. A regression test rejects a
22 m translated landmark pattern with unchanged image geometry and heading.
The synthetic two-sample recording fixture tests initialization, since it has
no previous field graph. Full-history tests cover actual recovery.

Full 425 s replay, Gaussian IMU factors, 2 s optimization window:

| Spline spacing | Solver | Checkpoints | >1 m transitions within 0.5 s | Largest such transition | Stage time |
| --- | --- | ---: | ---: | ---: | ---: |
| 200 ms | 20 iterations, original stopping | 6/10 | 8 | 3.75 m | 123.21 s |
| 200 ms | 60 iterations, small-step stop disabled | 5/10 | 10 | 4.33 m | 132.25 s |
| 100 ms | 20 iterations, original stopping | 9/10 | 11 | 5.34 m | 132.78 s |
| 100 ms | 60 iterations, small-step stop disabled | 9/10 | 9 | 4.05 m | 132.56 s |

Paired finer-grid runs (both 60 iterations, small-step stop disabled):

| Spline spacing | Preintegration interval | Checkpoints | >1 m transitions within 0.5 s | Largest | Stage time |
| --- | --- | ---: | ---: | ---: | ---: |
| 100 ms | 50 ms | 8/10 | 15 | 4.76 m | 157.79 s |
| 50 ms | 50 ms | 10/10 | 4 | 4.21 m | 235.54 s |

The 50 ms run's checkpoint maxima are 0.420 m horizontal, 0.120 m height,
and 1.530° yaw. Early recovery remains at 00:39.524. Its four short-interval
transitions above 1 m end at 05:56.652 (1.09 m), 06:16.334 (3.74 m),
06:18.394 (4.21 m), and 06:20.155 (1.71 m). The last is Tracking-to-Tracking;
the others are LostTrack-to-Tracking. Passing checkpoints still does not meet
the continuity requirement. Production grid defaults have not been changed.

The experimental full-covariance pose-revision gate at 50/50 ms preserves
10/10 checkpoints and reduces short-interval transitions above 1 m from four
to two: 1.089 m in 94 ms at 05:56.652 and 1.264 m in 80 ms at 06:17.266.
Stage time is 239.88 s versus 235.54 s without the gate. Field output counts
fall from 9,954 to 9,915 and Tracking counts from 9,406 to 9,346; these are
sample counts, not time-weighted availability. Maximum checkpoint errors are
0.377 m horizontal, 0.120 m height, and 1.276° yaw.

Across output gaps, the largest reported displacement remains 2.207 m over
1.549 s (06:34.065–06:35.614); another reported gap spans 2.122 s. These
transitions must not be confused with proof of uninterrupted localization.
The gate rejects 56 updates, with squared scores from 18.46 to 227.17 against
16.81. It checks the previous published pose in its right-local covariance,
not an independent-measurement NIS; shared evidence prevents treating the two
posteriors as independent. The additional check is skipped once that pose
leaves the retained spline window, although marginal priors remain.

This experiment still fails continuity: uncertain historical poses can admit
large revisions. Tightening the threshold alone is not an established fix.
Report: `/tmp/opencode/relocalization-innovation-50.json`; rejection details
are in the adjacent `.log`. No tested configuration is deployment-validated.

Follow-up diagnostics (`relocalization-diagnosis-50.log`) identify actual
same-time pose revisions of 1.164 m and 1.278 m at the two remaining jumps.
Both follow gaps in accepted landmark observations; their covariance scores
are below the gate. This is evidence of historical correction, not high speed.
The diagnostic also exposes an internal/publication mismatch: two solves can
run inside one localization update, and the first used to overwrite the gate's
reference before publication. The reference now advances only when Localization
returns a field estimate; an internal solve or field-unavailable output cannot
replace it.

The publication-boundary fix alone reproduces 10/10 checkpoints, two jumps,
9,915 field outputs and 9,346 Tracking outputs. It fixes the reference semantics
but does not eliminate either recorded jump. The longest same-epoch field
output gap is 2.122 s (04:36.483–04:38.605). Report:
`/tmp/opencode/relocalization-publication-50.json`. Timing from this run is
not an isolated benchmark because compilation and another replay overlapped.

Another delay was enforced by lifecycle dispatch: failed local association
could only use heading-guided global fallback after the two-second Tracking
timeout. Fallback now runs immediately when local matching fails, using fresh
exposure attitude and the existing heading reference. Localization admits these
observations into its retained graph even while Tracking; it never bootstraps
an already initialized graph. Regression coverage checks rejection of a 22 m
translated pattern followed by acceptance of a consistent global observation.
At 50/50 ms, immediate fallback passes 10/10 checkpoints but still has two
short-interval jumps: 1.082 m in 166 ms at 04:39.415 and 1.269 m in 80 ms at
06:17.266. The earlier 05:56.652 jump disappears, but the new 04:39 event means
this is not a continuity fix. Its historical revision is 0.965 m with a score
of 5.556, below the gate. A single global frame can still produce a large,
statistically admissible correction before subsequent tracking observations.
Field/Tracking output counts improve to 9,962/9,473, from 9,915/9,346. The
longest same-epoch field output gap remains 2.122 s. Report:
`/tmp/opencode/relocalization-early-recovery-50.json`. Stage time is 264.17 s,
but overlapping compilation/replay makes it unsuitable as an isolated runtime
comparison. The completed 200/100 ms runs of the same implementation fail:

| Solver | Checkpoints | >1 m transitions within 0.5 s | Largest | Stage time | Field / Tracking outputs |
| --- | ---: | ---: | ---: | ---: | ---: |
| Production (20 iterations) | 6/10 | 9 | 6.19 m | 143.50 s | 8,528 / 7,267 |
| 60 iterations, small-step stop disabled | 6/10 | 10 | 6.90 m | 157.40 s | 8,532 / 7,369 |

Both fail 03:58.487 and 04:06.126 with about 2.9–3.1 m horizontal error and
LostTrack, and have missing field poses at 06:37.793 and 07:02.793. Their
longest same-epoch field output gaps are 29.14 s and 28.03 s respectively.
The largest short-interval corrections end at 04:28.978. Thus supporting the
200 ms configuration does not mean the recovery changes work adequately with
it. Reports: `/tmp/opencode/relocalization-early-recovery-200{,-precise}.json`.
These retain 100 ms preintegration; comparison with 50/50 ms changes both grid
and preintegration interval, not spline spacing alone.

The 100 ms variants still fail 07:02.793: approximately 1.35 m horizontal and
0.76 m height error, despite reporting Tracking. The combined variant's largest
transition is at 03:06.753–03:06.865. Its position covariance root-trace falls
from 1.98 m to 0.32 m: an uncertain prior is being revised, not a new epoch.
This is not sufficient to satisfy publication continuity.

The replay reports the 30 largest transitions, including transitions across
missing-output gaps; resets are excluded. It reports field/tracking estimate
counts as well, so a lower jump count cannot conceal loss of availability.
The earlier jump counter required adjacent outputs with field poses; the new
counter also bridges intervening outputs without a field pose. Counts from the
two definitions are not directly comparable.

Rejected experiments: a 0.25 m historical-revision limit reduced short-interval
jumps to zero but passed only 2/10 checkpoints; robust full inertial residuals
passed 6/10 and cost about 194 s; freezing the heading reference did not improve
those results. The revision limit and robust inertial changes were removed.
Sensor ablations are diagnostic only: disabling acceleration passed 8/10 but
still jumped; disabling VO was substantially worse. These do not establish
faulty IMU measurements.

Replay modes `precise`, `grid-100`, and `grid-100-precise` reproduce the table.
`grid-100-interval-50` and `grid-50-precise` compare the finer grids at a common
50 ms preintegration interval. `motion` reports raw motion extremes and the
recorded checkpoint IMU-to-field heading offsets. `no-accel`, `no-vo`, and
`fixed-heading` are diagnostic ablations, not recommended configurations.

Reports are `/tmp/opencode/relocalization-{seeded,precise,grid-100,grid-100-precise}.json`.

The following sections describe the earlier, reinitializing implementation.

Run from the repository root:

```sh
cargo run -p localization-3d --example relocalization_replay -- recovered.mcap 425
```

JSON goes to stdout; diagnostics go to stderr. The optional third argument is
`iterations-N`, `convergent` (30 iterations), `wide-heading`, or `strict-classes`.
`baseline` (the default) uses the **current** compiled configuration and code;
it does not restore the original branch implementation.

The harness replays sensor delivery order through production association and
localization, using recorded solve events and epoch changes. Damping-exit epoch
changes discard localization and pending measurements. Field dimensions are
those embedded in this recording. Build once, then run binaries sequentially
without concurrent compilation when comparing runtime. Reported stage times
exclude MCAP loading and include host scheduling delays.

The ten user-validated timestamps are regression references, not independent
ground truth. A checkpoint passes with horizontal error ≤1 m, absolute height
error ≤0.5 m, absolute yaw error ≤15°, estimate age ≤100 ms, and Tracking state.
The replay selects the nearest estimate to each checkpoint. Missing field poses
fail. Lifecycle transitions and within-epoch jumps over 1 m are also reported;
passing checkpoints alone cannot establish correctness between them.

This is a deterministic approximation of asynchronous execution, not a
bit-identical reproduction. It uses received sensor caches and the recorded
solve schedule rather than modeling a changed solver's delivery latency.

## Recovery changes

- Try the coherent motion posterior first during visual loss, then use the
  existing global matcher if local association cannot disambiguate the image.
- Fuse unambiguous one-/two-feature tracking observations into the existing
  trajectory; global initialization still requires at least three features.
- Keep existing covariance, age, lifecycle, heading and visual consistency gates.
  No new contact constraints are introduced; Damping remains an explicit reset.
- Raise the solver iteration ceiling from 10 to 20 so the early recovery
  candidates can converge. Normal solves still terminate on convergence.

The global fallback remains a bounded sampled search, not a proof of global
uniqueness. This change does not maintain multiple persistent trajectories.

## Recording results

Sequential host runs over the first 425 seconds of `recovered.mcap`, using
optimized dev builds and no concurrent compilation:

| Metric | Original (10 iterations) | Local-first + sparse updates (20 iterations) |
| --- | ---: | ---: |
| Checkpoints passed | 8/10 | 10/10 |
| First recovery after 00:37.60 reset | 00:46.02 (wrong heading) | 00:39.52 |
| Association + localization time | 95.76 s | 117.18 s (+22.4%) |
| Association time | 1.48 s | 0.40 s |
| Localization median / p95 | 5.73 / 22.09 ms | 8.38 / 24.05 ms |

The original fails 06:00.263 (LostTrack and height error) and lacks a field pose
at 07:02.793. Final checkpoint errors:

| Timestamp | Horizontal (m) | Absolute height (m) | Absolute yaw (°) |
| --- | ---: | ---: | ---: |
| 02:42.236 | 0.062 | 0.004 | 0.150 |
| 02:50.157 | 0.467 | 0.055 | 2.733 |
| 03:58.487 | 0.013 | 0.001 | 0.019 |
| 04:06.126 | 0.021 | 0.000 | 0.367 |
| 04:44.751 | 0.102 | 0.000 | 1.192 |
| 04:56.848 | 0.146 | 0.000 | 1.022 |
| 05:47.541 | 0.180 | 0.001 | 1.621 |
| 06:00.263 | 0.379 | 0.096 | 1.671 |
| 06:37.793 | 0.357 | 0.043 | 0.024 |
| 07:02.793 | 0.235 | 0.183 | 2.820 |

Seven within-epoch jumps over 1 m remain in the final replay; recovery at
00:39.52 and checkpoint success do not prove continuous trajectory accuracy.
The early recovered pose is approximately (-8.765, 6.194, 0.551) m with yaw
-42.35°, consistent with the earlier geometric recovery hypothesis.

Rejected experiments: 15 iterations passed the late checkpoints but retained
the early wrong-heading trap; the joint-tilt gate plus warm-start prototype
passed only 6/10. Thirty iterations passed all checkpoints but was unnecessary
for early convergence. Both prototypes were removed from the final code.

Raw reports from this session are in
`/tmp/opencode/relocalization-baseline-clean.json` and
`/tmp/opencode/relocalization-simple-20.json` (adjacent `.log` files hold
diagnostics). These temporary files are not repository fixtures.

Validation: 51 association unit tests, 37 localization unit tests (including
unsupported-flight recovery), and the shared association-geometry test pass;
the replay example builds. The optional legacy `recorded_flip_from_mcap` test
fails decoding `RecordedEstimate` at `recording_tests.rs:375` with
`InvalidOptionTag`; its older recording schema does not match this MCAP. The
new replay decodes the current estimate schema successfully. Hardware timing
and continuous trajectory ground truth were not checked.
