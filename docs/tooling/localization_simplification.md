# Localization Simplification Checklist

Review scope: current 3D localization, field-mark association, stereo VO, and their
simulator/fixture integration. Existing unrelated worktree changes are preserved.

Acceptance rule: evaluate findings individually. Retain a change only when it
simplifies the code without reducing correctness or performance. A checked item
means evaluated, not necessarily changed. Outcomes are `fixed`, `rejected`, or
`deferred`; verification is recorded below. No algorithm rewrite is implied.

Outcome: **53 addressed, 18 rejected**, including two discarded implementation
trials. Documentation-only fixes are labeled explicitly; their proposed runtime
or configuration changes were not applied.

## Findings

- [x] 1. **Fixed:** freshness is enforced once in shared ingestion; 46 localization unit tests pass, including expired Startup regression.
- [x] 2. **Rejected:** unifying predicates changes accepted covariance sets or eigensolver convergence/performance. Keep strict producer checks and bounded tolerant consumer checks; equivalence is not established.
- [x] 3. **Fixed (documentation):** state the block-independent orientation-noise contract. Rejecting previously accepted matrices or redesigning configuration is not a behavior-preserving simplification.
- [x] 4. **Rejected:** both callers already enforce ordering and have reset coverage. Moving it requires a new outcome protocol across cache/exact-sample ingestion and failure paths; no smaller safe implementation established.
- [x] 5. **Fixed:** derive lock status from timestamps and derive Default; all 46 localization unit tests pass.
- [x] 6. **Rejected:** deriving the deadline on every select iteration also repeats work on unrelated high-rate sensor arrivals. Existing cache maintenance is tested; strict performance non-regression is not established for this cleanup.
- [x] 7. **Fixed:** use one From conversion for direct and mailbox results; all 46 localization unit tests pass.
- [x] 8. **Rejected:** explicit reset is necessary for delayed checkpoints; avoiding only the same-timestamp duplicate adds another conditional correction protocol. Keep tested exact/deferred paths.
- [x] 9. **Rejected:** the proposed reuse constructs backend orientation in f32 instead of the current f64. Keep numerical precision; initialization is not a hot path.
- [x] 10. **Fixed:** remove four unused output fields; localization tests pass, simulator caller verification included in final checks.
- [x] 11. **Rejected:** accessors repeat pose composition for multiple consumers and require test/API churn. Cached result construction is authoritative in production; keep it.
- [x] 12. **Fixed:** reuse existing field_to_robot helper after the timestamp guard; 46 localization tests pass.
- [x] 13. **Rejected:** removing checks worsens parameter-specific error messages for no meaningful complexity reduction; preserve validation diagnostics.
- [x] 14. **Rejected:** moving two three-line private accessors into a new public projection API expands the shared surface and touches unrelated callers. The expression is already direct.
- [x] 15. **Fixed:** use Time::abs_diff directly; 46 localization tests pass.
- [x] 16. **Fixed:** delete unused constructor, position/epoch getters, and Extrinsics symbol; 116 localization/backend tests pass.
- [x] 17. **Rejected:** projection-before-retention and raw-image retention use different representations. Sharing the small loop requires generic accessors or extra intermediate data; not simpler than two direct policies.
- [x] 18. **Fixed:** document scalar, 2D, and joint gate semantics without changing thresholds.
- [x] 19. **Fixed:** distinguish independent point covariance from shared-error certification.
- [x] 20. **Fixed:** name anchor/predicted poses explicitly and document their timestamps; no arithmetic changes.
- [x] 21. **Fixed:** document completion/pruning/abort semantics; no new error enum or search machinery.
- [x] 22. **Rejected:** current code replaced signs with TriangleConstraint records, including contrast bounds. Combining with order now needs optional seed entries or extra padding; retain compact precomputed arrays.
- [x] 23. **Fixed:** use named distance/tolerance fields; unchanged arithmetic, traversal, and two-float storage; association tests pass.
- [x] 24. **Fixed:** distinguish log likelihoods from normalized benefits with a move, not an allocation.
- [x] 25. **Fixed (documentation):** explain zero/dummy optimal-score equivalence. Keep zero rather than changing assignment solver paths/tie behavior without performance evidence.
- [x] 26. **Fixed:** name candidate cursors and document valid covariance prefixes; no search changes.
- [x] 27. **Fixed:** direct indexing for closed-enum/map-generated indices; association tests pass.
- [x] 28. **Fixed:** explicit accumulation loops, preserving order and fixture capacity.
- [x] 29. **Fixed:** reuse Rectangle::center; goalpost max-y convention unchanged.
- [x] 30. **Fixed:** share Intrinsic::is_valid at all three boundaries; added focal/center regression test.
- [x] 31. **Fixed:** share the factor's exact depth cutoff with lock/seed checks. All 168 projection/association/localization/backend unit tests pass (three runtime tests ignored).
- [x] 32. **Rejected after trial:** pairing measurements with taus changes element size, forcing constructors to allocate/copy the entire incoming measurement buffer instead of retaining it and allocating only taus. Reverted this trial; keep existing cached vectors/assertions.
- [x] 33. **Fixed:** explicit fallible key loop, reserved 21-entry selector, and tangent-layout comments; 70 backend tests pass.
- [x] 34. **Fixed:** assign interval indices once, derive start times; preserve previous-before-current and pre-origin filtering; 70 backend tests pass.
- [x] 35. **Fixed:** replace extension trait with a local function; backend tests pass.
- [x] 36. **Fixed:** bind state insertion before containment condition; backend tests pass.
- [x] 37. **Rejected:** retained-key decoding/sorting adds allocations and a separate traversal protocol to one-time bootstrap. It helps long histories but is not uniformly cheaper or simpler; no performance evidence supports the tradeoff here.
- [x] 38. **Fixed:** evaluate pose only in accelerometer branch; gyro/accelerometer tests pass.
- [x] 39. **Fixed:** remove middle GP constructor, preserving bridge options and noise scale; backend tests pass.
- [x] 40. **Fixed:** pass sigma rather than the entire factor; foot tests pass.
- [x] 41. **Fixed:** use Graph::iter and is_residual; backend tests pass.
- [x] 42. **Fixed:** use existing scalar-sigma constructor with identical whitening; backend tests pass.
- [x] 43. **Fixed:** remove dead reset-vector filtering; sensor cutoff filtering unchanged.
- [x] 44. **Rejected:** a generic helper would need context plumbing to preserve distinct diagnostic messages. Existing standard Cholesky operations are already direct; no useful reduction in conceptual complexity.
- [x] 45. **Rejected after trial:** one-shot interpolation consumes/scales its dynamic tangent buffer; borrowed GeodesicSpline::evaluate allocates a scaled copy to retain the spline. Keep the two-line one-shot implementation.
- [x] 46. **Fixed:** remove discarded left-RMSE traversal/conversions; caller still validates finite stereo metrics; six stereo unit tests pass.
- [x] 47. **Fixed:** reject underconstrained refinement instead of maintaining an unused success policy/helper; stereo tests pass.
- [x] 48. **Fixed:** remove six write-only diagnostic fields and benchmark vectors; numerical acceptance metrics retained; stereo tests pass.
- [x] 49. **Rejected:** combining separate sums changes floating-point accumulation order near acceptance thresholds. Preserve exact metrics rather than trade numerical behavior for two fields.
- [x] 50. **Rejected:** disparity diagnostics currently use a broader eligibility predicate than correspondence construction. Folding them together changes the reported population; keep semantics rather than silently redefine diagnostics in a cleanup.
- [x] 51. **Fixed:** select initial pose/fallback flag once and share refinement tail; same flags and diagnostic updates; stereo tests pass.
- [x] 52. **Fixed:** use a direct point-count accessor, avoiding debug point-cloud allocation; simulator tests pass.
- [x] 53. **Fixed:** name matched_pairs with source/target indices and retain score validation internally; stereo tests pass.
- [x] 54. **Fixed:** copy previous features once after either branch, preserving error/order semantics; stereo/simulator builds pass (inference test remains ignored).
- [x] 55. **Fixed:** remove constant image-name arguments; same validation and error text.
- [x] 56. **Fixed:** reuse sibling algebra-conversion helpers; stereo tests pass.
- [x] 57. **Fixed:** document translation-first left increments and the baseline chain rule; no numerical edits.
- [x] 58. **Fixed:** document distinct Huber, squared-error, and stereo-RMSE contracts; no metric changes.
- [x] 59. **Fixed (documentation/naming):** clarify full-pipeline timing and rename local publisher; retain existing wire topic for consumers/recordings.
- [x] 60. **Fixed:** borrow pose parameters through block_in_place; no clone required.
- [x] 61. **Fixed:** delete duplicate lambda-order predicate, preserving validation/errors.
- [x] 62. **Fixed:** use the original world point directly in synthetic LM fixture; six stereo tests pass (one inference test ignored).
- [x] 63. **Fixed:** alias the production class enum and reuse raw_detections while retaining exact preallocation and group order; simulator/report tests pass.
- [x] 64. **Rejected:** extracting a shared lifecycle adapter expands ownership/API and must preserve async publication/error ordering versus synchronous truth bookkeeping. Keep direct adapters until a smaller equivalent boundary is demonstrated.
- [x] 65. **Fixed:** return the measurement directly; pipeline failures still become reset-status measurements, initialization remains fallible.
- [x] 66. **Fixed:** keep current live pose in a local variable, not persistent state; simulator trajectory tests pass.
- [x] 67. **Fixed:** use FieldDimensions::goal_post with the same half/side order and Bevy signs; simulator builds/tests pass, GPU rendering remains unexecuted.
- [x] 68. **Rejected:** display and optical transforms are distinct, and scene.rs belongs to the binary rather than the library. Sharing the tiny basis/permutation needs new library exports without simplifying either transform; retain explicit conventions.
- [x] 69. **Fixed:** share finite half-open image bounds before/after noise; added explicit boundary/nonfinite regression test; simulator tests pass.
- [x] 70. **Fixed:** use existing wallclock-to-ROS conversion for nonnegative simulation timestamps; simulator tests pass.
- [x] 71. **Fixed:** remove impossible status counter, redundant detection bitmap, and discarded enumeration index; historical wire layout/landmark collision checks preserved; three recording tests pass (writing extractor not run).

## Verification

Baseline: 161 core unit tests passed, three runtime-characterization tests ignored.

Final package tests: 239 passed, six explicitly ignored, using:

```sh
cargo test --locked -p localization-3d -p localization-fagra -p field_mark_association -p projection -p stereo_visual_odometry -p localization_simulator --no-default-features --features stereo_visual_odometry/ort-webgpu --quiet
```

This includes recorded association fixtures, IMU spline integration, projection
integration, simulator library/binary tests, and stereo mathematics. Added checks
cover stale Startup frames, intrinsic validity, match filtering, and pixel bounds.
Two independent read-only reviews found no concrete correctness/performance
regressions in accepted changes. Targeted Rust formatting was applied only to
touched files.

The KITTI benchmark target also compiles without executing a dataset:

```sh
cargo test --locked -p stereo_visual_odometry --bench kitti_odometry --no-run --no-default-features --features ort-webgpu --quiet
```

`git diff --check` passes. Existing user changes were neither reset nor reverted.

No measured speedup is claimed. Accepted runtime edits retain operations/order or
remove unused work; changes with unproven numerical/allocation tradeoffs were
rejected. GPU rendering, neural inference, KITTI sequence execution, and the
fixture-writing MCAP extractor are not covered by the normal test run.
