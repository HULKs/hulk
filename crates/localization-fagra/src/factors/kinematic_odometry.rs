use coordinate_systems::Ground;
use fagra::{BlockId, EvaluationError, Factor, LinearizationSink, StateKey, StateStore};
use nalgebra::{Matrix2, RealField, Rotation2, SMatrix, Vector2};

use super::common;
use crate::variables::PoseControl;

/// Horizontal displacement in the previous leveled robot frame. Four controls
/// cover one segment; five cover adjacent segments. No independent yaw evidence:
/// the kinematic odometer obtains its heading from the same IMU as localization.
#[derive(Clone, Debug)]
pub struct KinematicOdometry<R: RealField + Copy = f64, const N: usize = 4> {
    pub controls: [StateKey<PoseControl<R>>; N],
    pub duration: R,
    pub previous_tau: R,
    pub current_tau: R,
    pub translation: linear_algebra::Vector2<Ground, R>,
    pub information_root: Matrix2<R>,
    pub huber_threshold: R,
}

pub type AdjacentKinematicOdometry<R = f64> = KinematicOdometry<R, 5>;

impl<R: RealField + Copy, const N: usize> KinematicOdometry<R, N> {
    fn validate(&self) -> Result<(), EvaluationError> {
        let offset = match N {
            4 => R::zero(),
            5 => R::one(),
            _ => return Err(EvaluationError::InvalidEvaluation),
        };
        if self.current_tau + offset <= self.previous_tau {
            return Err(EvaluationError::InvalidEvaluation);
        }
        common::unique(&self.controls)?;
        common::finite(
            self.translation
                .inner
                .iter()
                .chain(self.information_root.iter()),
        )?;
        common::positive(self.huber_threshold)?;
        Ok(())
    }

    fn prediction(
        &self,
        a: &nalgebra::Isometry3<R>,
        b: &nalgebra::Isometry3<R>,
    ) -> Result<(Vector2<R>, Matrix2<R>), EvaluationError> {
        let rotation = Rotation2::new(-common::heading(&a.rotation)?).into_inner();
        Ok((
            rotation * (b.translation.vector - a.translation.vector).xy(),
            rotation,
        ))
    }
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>, const N: usize> Factor<S>
    for KinematicOdometry<R, N>
{
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        self.validate()?;
        let first = common::spline(
            states,
            self.controls
                .first_chunk::<4>()
                .ok_or(EvaluationError::InvalidEvaluation)?,
            self.duration,
        )?;
        let a = first.pose(self.previous_tau)?;
        let b = if N == 4 {
            first.pose(self.current_tau)?
        } else {
            common::spline(
                states,
                self.controls
                    .last_chunk::<4>()
                    .ok_or(EvaluationError::InvalidEvaluation)?,
                self.duration,
            )?
            .pose(self.current_tau)?
        };
        let (prediction, _) = self.prediction(&a.inner, &b.inner)?;
        Ok(common::huber(
            &(self.information_root * (prediction - self.translation.inner)),
            self.huber_threshold,
        )?
        .0)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        self.validate()?;
        let first = common::spline(
            states,
            self.controls
                .first_chunk::<4>()
                .ok_or(EvaluationError::InvalidEvaluation)?,
            self.duration,
        )?;
        let linearized = first.linearize()?;
        let a = linearized.pose(self.previous_tau)?;
        let b = if N == 4 {
            linearized.pose(self.current_tau)?
        } else {
            common::spline(
                states,
                self.controls
                    .last_chunk::<4>()
                    .ok_or(EvaluationError::InvalidEvaluation)?,
                self.duration,
            )?
            .linearize()?
            .pose(self.current_tau)?
        };
        let (prediction, rotation) = self.prediction(&a.pose.inner, &b.pose.inner)?;
        let residual = self.information_root * (prediction - self.translation.inner);
        let scale = common::huber(&residual, self.huber_threshold)?.1;
        let mut ha = SMatrix::<R, 2, 6>::zeros();
        let mut hb = SMatrix::<R, 2, 6>::zeros();
        ha.fixed_view_mut::<2, 3>(0, 0).copy_from(
            &(Vector2::new(prediction.y, -prediction.x)
                * common::heading_jacobian(&a.pose.inner.rotation)?),
        );
        ha.fixed_view_mut::<2, 3>(0, 3).copy_from(
            &(-rotation
                * a.pose
                    .inner
                    .rotation
                    .to_rotation_matrix()
                    .matrix()
                    .fixed_rows::<2>(0)),
        );
        hb.fixed_view_mut::<2, 3>(0, 3).copy_from(
            &(rotation
                * b.pose
                    .inner
                    .rotation
                    .to_rotation_matrix()
                    .matrix()
                    .fixed_rows::<2>(0)),
        );
        let ha = self.information_root * ha * scale;
        let hb = self.information_root * hb * scale;
        let jacobians = std::array::from_fn(|i| {
            let mut j = SMatrix::<R, 2, 6>::zeros();
            if i < 4 {
                j += ha * a.jacobians[i];
            }
            if i >= N - 4 {
                j += hb * b.jacobians[i - (N - 4)];
            }
            j
        });
        common::emit(sink, &self.controls, &(residual * scale), &jacobians)
    }
}
