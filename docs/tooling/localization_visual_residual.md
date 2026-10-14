# Monotone visual bearing residual

## Contract

`localization-fagra::factors::FrameReprojections` retains its historical name, but
implements an **oriented-bearing residual**, not pinhole pixel error and not a
front/behind blend. Each observation emits three residual components. Use the same
factor in front of, beside, and behind the predicted camera. No image bounds,
visibility switch, depth clamping, or residual-model transition is involved.

This permits valid correspondences to correct an invalid predicted pose. It does
not certify correspondences, resolve field symmetry, or guarantee global solver
convergence. Association and lifecycle acceptance remain separate responsibilities.

## Definition and monotonicity

Let `q` be the fixed field landmark transformed through field alignment and the
trajectory into the optical camera frame (`x` right, `y` down, `z` forward).
For measured pixel `(u, v)` and current graph intrinsics:

```text
n = [(u - cx) / fx, (v - cy) / fy, 1]
b = n / ||n||                    observed unit bearing
d = q / ||q||                    predicted unit bearing
c = bᵀ d
w = angular_information_root     fixed 1 / sigma_theta
a = sqrt(3 / (2 + c))
r = w a (d - b)                  three-component residual
```

The unrobustified objective is:

```text
C = 1/2 ||r||² = 3 w² (1 - c) / (2 + c)
```

For angular separation `theta` in `[0, pi]`, `c = cos(theta)`:

```text
dC/dtheta = 9 w² sin(theta) / (2 + cos(theta))²
```

This derivative is strictly positive on `(0, pi)`. For a fixed observation,
intrinsics, weight, and nonzero range, increasing angular separation increases
cost. Crossing an image edge or `q.z = 0` introduces no discontinuity, singularity,
or lower-cost escape. The denominator lies in `[1, 3]`.

This is **observation-relative monotonicity**, not a rule that every outside-image
prediction costs more than every inside-image prediction. It also does not imply
that every component of a multivariable graph update is monotone.

Near the solution, `C = theta² / (2 sigma_theta²) + O(theta⁴)`. At an orthogonal
bearing, `C = 1.5 w²`; at the antipode, `C = 6 w²`. The full oriented difference
`d - b` is essential: a cross-product-only residual would also vanish at the antipode.

## Noise and runtime parameter meaning

The factor accepts one positive scalar `angular_information_root`, in inverse
radians. It represents isotropic angular noise. Three residual components do not
mean three independent image measurements: the observation is a direction on the
two-dimensional unit sphere.

The localization node keeps `visual_feature_noise_variance` in pixel² for existing
configuration and converts it once, when inserting the frame:

```text
f_reference = sqrt(fx_calibrated * fy_calibrated)
sigma_theta = sqrt(visual_feature_noise_variance) / f_reference
angular_information_root = f_reference / sqrt(visual_feature_noise_variance)
```

The geometric mean uses the frame's fixed calibrated focal lengths. It is a
near-optical-axis approximation and is **not** an exact anisotropic pixel-noise
likelihood, particularly far off axis or for unequal focal lengths. This scalar
weight stays fixed while optimizing intrinsics; changing a graph focal length
must not silently change the strength of the same measurement.

The observed bearing still depends on optimized focal lengths and optical centre,
and all those derivatives are included. Existing intrinsics priors remain active.

## Analytical Jacobians

With `e = d - b`, `s = 2 + c`, and identity matrix `I`:

```text
dr/dd = w a [ I - e bᵀ / (2s) ]
dr/db = w a [-I - e dᵀ / (2s) ]
dd/dq = (I - d dᵀ) / ||q||
db/dn = (I - b bᵀ) / ||n||

dn/d[fx, fy, cx, cy] =
    [-nx/fx,       0, -1/fx,     0]
    [     0, -ny/fy,     0, -1/fy]
    [     0,       0,     0,     0]
```

Compose `dr/dd * dd/dq` with the existing right-increment trajectory and field
alignment Jacobians. Compose `dr/db * db/dn` with the intrinsic Jacobian above.
In particular, **differentiate the directional weight `a`**. Freezing it changes
the objective's gradient and is incorrect.

## Robust loss

Apply the existing Huber loss to the norm of the complete whitened residual:

```text
t = ||r||
rho(t) = 1/2 t²                 if t <= delta
         delta (t - delta/2)    otherwise
```

The Huber objective remains monotone in angular error. It is continuously
differentiable, but not twice differentiable at its threshold. The raw bearing
residual is smooth throughout its valid range domain.

Linearization freezes only the Huber IRLS weight, applying it to both residual and
Jacobians. This preserves the robust objective gradient `Jᵀr`. Do not confuse that
IRLS weight with `a`, whose state dependence must be differentiated.

## Domain and acceptance rules

- Require finite observations/transforms/intrinsics, positive focal lengths,
  positive angular information, and a positive Huber threshold.
- `min_range` bounds **Euclidean camera-to-landmark distance**, not optical depth.
  The node currently uses 0.01 m. At or below it, bearing is undefined or numerically
  unsuitable: return `InvalidEvaluation`; never silently emit zero cost.
- A negative optical depth or an off-image bearing is valid during optimization.
- The exact antipode is a stationary maximum. Its first-order pose gradient is
  zero; this factor alone cannot guarantee escape. Recovery seeding can still be
  necessary. Other priors can also prevent a large correction.
- The node admits already-associated observations with negative predicted depth,
  while retaining finite-input and minimum-range checks.
- Restoring `Tracking` still requires the existing positive-depth and <=10 pixel
  RMS validation, visual freshness, and solver convergence. A finite angular
  objective is not permission to accept a behind-camera final solution.

## Regression checks

`crates/localization-fagra/src/factors/tests.rs` checks f32/f64 residual Jacobians
against automatic differentiation, including trajectory, alignment, intrinsics,
and front/side/behind predictions. Robust-gradient checks verify the frozen IRLS
linearization. A 0-to-pi sweep verifies the closed-form cost and strict
monotonicity both with and without active Huber loss. Range and invalid-intrinsics
checks exercise the failure contract.

The localization node tests that behind-camera correspondences are admitted while
zero-range observations are rejected and behind-camera poses fail pixel validation.

```sh
cargo test -p localization-fagra
cargo test -p localization-3d --lib
cargo test -p localization_simulator --lib
```
