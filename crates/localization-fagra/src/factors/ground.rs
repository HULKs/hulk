use coordinate_systems::Robot;
use fagra::{
    BlockId, EvaluationError, FactorBatch, FactorSelection, LinearizationSink, StateKey, StateStore,
};
use linear_algebra::Point3;
use nalgebra::{RealField, SMatrix};

use super::common;
use crate::variables::PoseControl;

/// Sole positions from kinematics at one observation time.
#[derive(Clone, Debug)]
pub struct FootObservation<R: RealField + Copy = f64> {
    pub tau: R,
    pub left_sole: Point3<Robot, R>,
    pub right_sole: Point3<Robot, R>,
}

/// Two one-sided residuals per observation, penalizing soles below local z = 0.
/// Each row is `max(0, -height) / sigma`, where height is the sole's local z
/// after applying the interpolated robot pose. Emit both feet in one observation
/// scope. At the hinge (`height = 0`), use the inactive branch with zero Jacobian.
#[derive(Clone, Debug)]
pub struct FootGround<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    /// Positive height standard deviation in metres.
    pub sigma: R,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> FactorBatch<S> for FootGround<R> {
    type Scalar = R;
    type Factor = FootObservation<R>;

    fn visit_variables(&self, _factor: &Self::Factor, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(
        &self,
        states: &S,
        factors: FactorSelection<'_, Self::Factor>,
    ) -> Result<R, EvaluationError> {
        if factors.is_empty() {
            return Ok(R::zero());
        }
        let inverse_sigma = common::positive(self.sigma)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let mut cost = R::zero();
        for (_, observation) in factors {
            let pose = spline.pose(observation.tau)?;
            let (residual, _) = foot_residuals(&pose.inner, observation, inverse_sigma)?;
            cost += common::cost(&residual)?;
        }
        common::checked_cost(cost)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        factors: FactorSelection<'_, Self::Factor>,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        if factors.is_empty() {
            return Ok(());
        }
        let inverse_sigma = common::positive(self.sigma)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let mut linearized = None;
        for (id, observation) in factors {
            // Recheck activity and input validity at the current state. Inactive
            // rows contribute no information; their factor scope still gets visited.
            let value = spline.pose(observation.tau)?;
            let (residual, active) = foot_residuals(&value.inner, observation, inverse_sigma)?;
            if !active[0] && !active[1] {
                sink.factor(id, |_| Ok(()))?;
                continue;
            }
            let prepared = match linearized {
                Some(ref prepared) => prepared,
                None => linearized.insert(spline.linearize()?),
            };
            let pose = prepared.pose(observation.tau)?;
            let rotation = pose.pose.inner.rotation.to_rotation_matrix().into_inner();
            let mut h = SMatrix::<R, 2, 6>::zeros();
            for (row, sole) in [observation.left_sole, observation.right_sole]
                .iter()
                .enumerate()
            {
                if active[row] {
                    h.row_mut(row).copy_from(
                        &(common::point_jacobian(rotation, sole.inner.coords).row(2)
                            * -inverse_sigma),
                    );
                }
            }
            let jacobians = pose.jacobians.map(|j| h * j);
            sink.factor(id, |sink| {
                common::emit(sink, &self.controls, &residual, &jacobians)
            })?;
        }
        Ok(())
    }
}

fn foot_residuals<R: RealField + Copy>(
    pose: &nalgebra::Isometry3<R>,
    observation: &FootObservation<R>,
    inverse_sigma: R,
) -> Result<(nalgebra::Vector2<R>, [bool; 2]), EvaluationError> {
    let mut residual = nalgebra::Vector2::zeros();
    let mut active = [false; 2];
    for (row, sole) in [observation.left_sole, observation.right_sole]
        .iter()
        .enumerate()
    {
        common::finite(sole.inner.coords.iter())?;
        let height = (pose * sole.inner).z;
        if !height.is_finite() {
            return Err(EvaluationError::InvalidEvaluation);
        }
        active[row] = height < R::zero();
        if active[row] {
            residual[row] = -height * inverse_sigma;
        }
    }
    Ok((residual, active))
}
