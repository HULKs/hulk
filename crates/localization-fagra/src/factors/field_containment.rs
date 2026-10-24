use coordinate_systems::Field;
use fagra::{
    BlockId, EvaluationError, Factor, JacobianBlock, LinearizationSink, StateKey, StateStore,
};
use linear_algebra::Vector2;
use nalgebra::{RealField, SMatrix};

use super::common;
use crate::variables::{FieldAlignment, PoseControl};

/// Two one-sided penalties for exceeding the field's x/y limits.
/// After composing the evaluated spline position at `tau` with field alignment, each coordinate `x`
/// contributes `(x - limit) / sigma` above its positive limit, `(x + limit) / sigma`
/// below its negative limit, and zero inside. At either boundary use the inactive
/// branch with zero Jacobian.
#[derive(Clone, Debug)]
pub struct FieldContainment<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    pub tau: R,
    pub alignment: StateKey<FieldAlignment<R>>,
    /// Positive half-extents of the allowed region, including the border, in metres.
    pub half_extents: Vector2<Field, R>,
    /// Positive distance standard deviation in metres.
    pub sigma: R,
}

impl<R, S> Factor<S> for FieldContainment<R>
where
    R: RealField + Copy,
    S: StateStore<PoseControl<R>> + StateStore<FieldAlignment<R>>,
{
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
        visitor(self.alignment.block_id());
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        let inverse_sigma = self.validate()?;
        let pose = common::spline(states, &self.controls, self.duration)?.pose(self.tau)?;
        let alignment = &states.get(self.alignment)?.local_to_field.inner;
        let position = alignment * nalgebra::Point2::from(pose.inner.translation.vector.xy());
        let (residual, _) = self.residual(position.coords, inverse_sigma)?;
        common::cost(&residual)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        let inverse_sigma = self.validate()?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let pose = spline.linearize()?.pose(self.tau)?;
        let alignment = &states.get(self.alignment)?.local_to_field.inner;
        let local = pose.pose.inner.translation.vector.xy();
        let position = alignment * nalgebra::Point2::from(local);
        let (residual, slopes) = self.residual(position.coords, inverse_sigma)?;
        let rotation = alignment.rotation.to_rotation_matrix().into_inner();
        let whitening = nalgebra::Matrix2::from_diagonal(&slopes);
        let mut ha = SMatrix::<R, 2, 3>::zeros();
        ha.column_mut(0)
            .copy_from(&(rotation * nalgebra::Vector2::new(-local.y, local.x)));
        ha.fixed_view_mut::<2, 2>(0, 1).copy_from(&rotation);
        let ha = whitening * ha;
        let mut hp = SMatrix::<R, 2, 6>::zeros();
        hp.fixed_view_mut::<2, 3>(0, 3).copy_from(
            &(whitening
                * rotation
                * pose
                    .pose
                    .inner
                    .rotation
                    .to_rotation_matrix()
                    .matrix()
                    .fixed_rows::<2>(0)),
        );
        let jacobians = pose.jacobians.map(|j| hp * j);
        let blocks: [_; 5] = std::array::from_fn(|i| {
            if i < 4 {
                JacobianBlock::new(self.controls[i], &jacobians[i])
            } else {
                JacobianBlock::new(self.alignment, &ha)
            }
        });
        sink.residual(&residual, &blocks)
    }
}

impl<R: RealField + Copy> FieldContainment<R> {
    fn validate(&self) -> Result<R, EvaluationError> {
        common::positive(self.half_extents.inner.x)?;
        common::positive(self.half_extents.inner.y)?;
        common::positive(self.sigma)
    }

    fn residual(
        &self,
        position: nalgebra::Vector2<R>,
        inverse_sigma: R,
    ) -> Result<(nalgebra::Vector2<R>, nalgebra::Vector2<R>), EvaluationError> {
        common::finite(position.iter())?;
        let mut residual = nalgebra::Vector2::zeros();
        let mut slopes = nalgebra::Vector2::zeros();
        for i in 0..2 {
            let limit = self.half_extents.inner[i];
            if position[i] > limit {
                residual[i] = (position[i] - limit) * inverse_sigma;
                slopes[i] = inverse_sigma;
            } else if position[i] < -limit {
                residual[i] = (position[i] + limit) * inverse_sigma;
                slopes[i] = inverse_sigma;
            }
        }
        Ok((residual, slopes))
    }
}
