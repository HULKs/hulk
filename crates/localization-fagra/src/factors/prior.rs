use fagra::{
    BlockId, EvaluationError, Factor, JacobianBlock, LinearizationSink, StateKey, StateStore,
    Variable,
};
use nalgebra::{RealField, SMatrix};

use super::common;
use crate::variables::{CameraIntrinsics, PoseControl, TrajectoryState};

/// Initial local-frame anchor on evaluated pose and velocity, not on a control pose.
/// Raw residual: `reference.local(spline.state(tau))`, ordered rotation, velocity,
/// position. Velocity is the derivative of the position spline, not an independent state.
/// Whiten this residual and its right-increment Jacobian with `information_root`.
#[derive(Clone, Debug)]
pub struct TrajectoryPrior<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    pub tau: R,
    pub reference: TrajectoryState<R>,
    pub information_root: SMatrix<R, 9, 9>,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> Factor<S> for TrajectoryPrior<R> {
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::finite(self.information_root.iter())?;
        let state = common::spline(states, &self.controls, self.duration)?.state(self.tau)?;
        let error = self.error(&state)?;
        common::cost(&(self.information_root * error))
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::finite(self.information_root.iter())?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let sample = spline.linearize()?.state(self.tau)?;
        let error = self.error(&sample.state)?;
        let derivative = self.information_root * TrajectoryState::right_jacobian_inverse(&error);
        let jacobians = sample.jacobians.map(|j| derivative * j);
        common::emit(
            sink,
            &self.controls,
            &(self.information_root * error),
            &jacobians,
        )
    }
}

impl<R: RealField + Copy> TrajectoryPrior<R> {
    fn error(
        &self,
        state: &TrajectoryState<R>,
    ) -> Result<nalgebra::SVector<R, 9>, EvaluationError> {
        common::finite(
            self.reference
                .pose
                .inner
                .translation
                .vector
                .iter()
                .chain(self.reference.velocity.inner.iter()),
        )?;
        common::rotation_log(
            &(self.reference.pose.inner.rotation.inverse() * state.pose.inner.rotation),
        )?;
        let error = self.reference.local(state);
        common::finite(error.iter())?;
        Ok(error)
    }
}

/// Calibration anchor; residual order is `[fx, fy, cx, cy]`.
/// Raw residual: current calibration minus reference calibration, in pixels.
#[derive(Clone, Debug)]
pub struct CameraIntrinsicsPrior<R: RealField + Copy = f64> {
    pub intrinsics: StateKey<CameraIntrinsics<R>>,
    pub reference: CameraIntrinsics<R>,
    pub information_root: SMatrix<R, 4, 4>,
}

impl<R: RealField + Copy, S: StateStore<CameraIntrinsics<R>>> Factor<S>
    for CameraIntrinsicsPrior<R>
{
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        visitor(self.intrinsics.block_id());
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::finite(self.information_root.iter())?;
        common::cost(
            &(self.information_root * (states.get(self.intrinsics)?.log() - self.reference.log())),
        )
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::finite(self.information_root.iter())?;
        let residual =
            self.information_root * (states.get(self.intrinsics)?.log() - self.reference.log());
        sink.residual(
            &residual,
            &[JacobianBlock::new(self.intrinsics, &self.information_root)],
        )
    }
}
