# Recorded Association Fixture

Run with `cargo test -p field_mark_association --test association_recording_fixtures`.

`association_fixtures.json` is unchanged historical sensor data. Its five expected
correspondences came from the retired pose-fitting associator, not a certificate
from the current similarity-fit matcher.

The Ground-to-Local adapter is valid for this single recorded frame: both have
the same horizontal plane. Independent ray/plane intersection and planar fitting
give these measurements:

| Quantity | Measurement |
| --- | --- |
| Robot height above the plane | 0.2992 m |
| Camera height above the plane | 0.6314 m |
| Diagnostic free-scale fit | 1.0242, implying 0.6467 m camera height |
| Fixed-height rigid-fit residual | 0.1499 m RMS, 5.310 px RMS |

Global association must recover all five correspondences with default parameters,
and reject excessive uncertainty (10 px pixel sigma, 0.1 rad tilt sigma, squared
gate 100), which brings these rays' uncertainty to the horizon.

`support/geometry_oracle.rs` constructs an independent tracking prior from the labels,
holding measured height and tilt fixed. The tracking test requires **all five original
exact correspondences** with default association parameters. No fitted scale is supplied
to association or localization, and no production code reads this oracle.

The oracle prior covariance comes from the planar fit, not an arbitrary near-zero
diagonal. With N=5 points and three fitted parameters, isotropic residual variance
is `SSE / (2N - 3) = 0.01604 m^2`. Centered translation and yaw standard deviations
are 0.0566 m and 0.0526 rad. Their joint covariance is transformed to the robot's
right tangent, preserving yaw/translation lever-arm correlations. Height and tilt
remain fixed in this fit; production's normal measurement floors still apply.

This matters for the recorded goalpost at pixel `[453.0, 234.5]`: the fixed-height
prior predicts `[461.964, 237.791]`, a 9.549 px residual. The previous `1e-6 * I`
oracle covariance falsely claimed 1 mm translation precision and gave squared
Mahalanobis error 13.187, outside the default 9.21 gate. The residual-derived
covariance gives 1.477 with the same pose, detections, and production parameters.
The test still requires all five exact matches, bounds the original fit residuals,
and checks that the prior covariance is positive semidefinite without introducing
height or tilt freedom.

This fixture covers global association and prior-based tracking, not end-to-end
localization bootstrap. Preserve it when adding independently reviewed recordings.
