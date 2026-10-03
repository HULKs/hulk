"""GTSAM 4.2 reference for the Rust manifold preintegrator (prints its fixture).

uv run --no-project --python 3.11 --with gtsam==4.2 --with 'numpy<2' \
    python docs/tooling/preintegration_reference.py

The wheel uses tangent preintegration. Use native single-sample PIMs (whose
means are exact), convert their covariance chart, then compose using numerical
derivatives of GTSAM NavState. This avoids assuming that tangent multi-step
integration is identical to manifold integration.
"""

import gtsam
import numpy as np


def derivative(function, dimension):
    h = 1e-5
    return np.column_stack(
        [
            (
                function(np.eye(dimension)[i] * h)
                - function(-np.eye(dimension)[i] * h)
            )
            / (2 * h)
            for i in range(dimension)
        ]
    )


def compose(a, b, dt):
    r = a.attitude().matrix()
    return gtsam.NavState(
        a.attitude().compose(b.attitude()),
        a.position() + a.velocity() * dt + r @ b.position(),
        a.velocity() + r @ b.velocity(),
    )


def reference():
    params = gtsam.PreintegrationParams.MakeSharedU(9.81)
    params.setGyroscopeCovariance(np.eye(3) * 2e-5)
    params.setAccelerometerCovariance(np.eye(3) * 0.09)
    params.setIntegrationCovariance(np.eye(3) * 1e-8)
    bias = gtsam.imuBias.ConstantBias(
        np.array([0.1, -0.2, 0.05]), np.array([0.02, -0.01, 0.03])
    )
    state = gtsam.NavState()
    covariance = np.zeros((9, 9))
    elapsed = 0.0
    for i in range(40):
        dt = [0.001, 0.002, 0.003, 0.004][i % 4]
        gyro = np.array([2.0 + elapsed, -1.0 + 2 * elapsed, 3.0 - elapsed])
        force = np.array(
            [0.5 - elapsed, 0.2 + elapsed, 9.81 + (20.0 if i == 17 else 0.0)]
        )
        pim = gtsam.PreintegratedImuMeasurements(params, bias)
        pim.integrateMeasurement(force, gyro, dt)
        step = gtsam.NavState(pim.deltaRij(), pim.deltaPij(), pim.deltaVij())
        theta = gtsam.Rot3.Logmap(step.attitude())
        # Tangent PIM [log(R), p_start, v_start] -> NavState right-local chart.
        chart = np.zeros((9, 9))
        chart[:3, :3] = derivative(
            lambda x: gtsam.Rot3.Logmap(
                step.attitude().between(gtsam.Rot3.Expmap(theta + x))
            ),
            3,
        )
        chart[3:6, 3:6] = step.attitude().matrix().T
        chart[6:9, 6:9] = step.attitude().matrix().T
        result = compose(state, step, dt)
        a = derivative(
            lambda x: result.localCoordinates(
                compose(state.retract(x), step, dt)
            ),
            9,
        )
        b = derivative(
            lambda x: result.localCoordinates(
                compose(state, step.retract(x), dt)
            ),
            9,
        )
        covariance = (
            a @ covariance @ a.T
            + b @ chart @ pim.preintMeasCov() @ chart.T @ b.T
        )
        state = result
        elapsed += dt
    # NavState [R, P_end, V_end] -> Rust [R, V_start, P_start].
    chart = np.zeros((9, 9))
    chart[:3, :3] = np.eye(3)
    chart[3:6, 6:9] = state.attitude().matrix()
    chart[6:9, 3:6] = state.attitude().matrix()
    covariance = chart @ covariance @ chart.T
    mean = np.concatenate(
        [
            gtsam.Rot3.Logmap(state.attitude()),
            state.velocity(),
            state.position(),
        ]
    )
    print(" ".join(f"{v:.17e}" for v in mean))
    for row in covariance:
        print(" ".join(f"{v:.17e}" for v in row))


if __name__ == "__main__":
    reference()
