use coordinate_systems::Robot;
use fagra::{
    BlockId, EvaluationError, Factor, FactorBatch, FactorSelection, LinearizationSink, StateKey,
    StateStore, Variable,
};
use linear_algebra::Isometry3;
use nalgebra::{RealField, SMatrix, SVector};

use super::common;
use crate::variables::PoseControl;

/// Relative robot motion between two observation times.
///
/// With interpolated robot-to-local poses `T_previous` and `T_current`, predicted
/// motion is `T_previous⁻¹ * T_current`. Raw residual is the SE(3) logarithm
/// `Log(current_to_previous⁻¹ * predicted_motion)`, ordered rotation xyz then
/// translation xyz. This is the coupled SE(3) log, not independent rotation and
/// translation subtraction. Measurement noise uses these same residual coordinates.
#[derive(Clone, Debug)]
pub struct VisualOdometryObservation<R: RealField + Copy = f64> {
    /// Normalized time in the previous observation's interval.
    pub previous_tau: R,
    /// Normalized time in the current observation's interval.
    pub current_tau: R,
    /// Current robot frame to previous robot frame. The `Robot` marker identifies
    /// the body axes; observation times distinguish the two frame instances.
    pub current_to_previous: Isometry3<Robot, Robot, R>,
}

/// Batched robust 6D relative-pose observations sharing one interval's spline preparation.
///
/// Each payload must satisfy `previous_tau < current_tau`. Each observation has
/// its own factor identity and Huber weight on the complete whitened 6D residual.
/// See [`VisualOdometryObservation`] for the residual direction and coordinates.
#[derive(Clone, Debug)]
pub struct VisualOdometry<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    /// Residual coordinate order: rotation xyz (radians), translation xyz (metres).
    pub information_root: SMatrix<R, 6, 6>,
    pub huber_threshold: R,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> FactorBatch<S> for VisualOdometry<R> {
    type Scalar = R;
    type Factor = VisualOdometryObservation<R>;

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
        validate(&self.information_root, self.huber_threshold)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let mut cost = R::zero();
        for (_, observation) in factors {
            if observation.previous_tau >= observation.current_tau {
                return Err(EvaluationError::InvalidEvaluation);
            }
            let (error, _) = error(
                &spline.pose(observation.previous_tau)?.inner,
                &spline.pose(observation.current_tau)?.inner,
                observation,
            )?;
            cost += common::huber(&(self.information_root * error), self.huber_threshold)?.0;
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
        validate(&self.information_root, self.huber_threshold)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let linearized = spline.linearize()?;
        for (id, observation) in factors {
            if observation.previous_tau >= observation.current_tau {
                return Err(EvaluationError::InvalidEvaluation);
            }
            let a = linearized.pose(observation.previous_tau)?;
            let b = linearized.pose(observation.current_tau)?;
            let (error, delta) = error(&a.pose.inner, &b.pose.inner, observation)?;
            let (residual, ha, hb) =
                local_model(error, &delta, &self.information_root, self.huber_threshold)?;
            let jacobians = std::array::from_fn(|i| ha * a.jacobians[i] + hb * b.jacobians[i]);
            sink.factor(id, |sink| {
                common::emit(sink, &self.controls, &residual, &jacobians)
            })?;
        }
        Ok(())
    }
}

/// Relative-pose constraint spanning two adjacent, equally spaced knot intervals.
///
/// `previous_tau` uses controls 0..4 and `current_tau` uses controls 1..5. Require
/// strictly increasing physical observation times: `1 + current_tau > previous_tau`.
/// Use the residual of [`VisualOdometryObservation`] and one Huber weight for all
/// six whitened rows. Sum contributions from both evaluations for each of the
/// three shared controls into one Jacobian block per control before emission.
#[derive(Clone, Debug)]
pub struct AdjacentVisualOdometry<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 5],
    /// Duration of each interval, not their sum.
    pub duration: R,
    pub observation: VisualOdometryObservation<R>,
    /// Residual coordinate order: rotation xyz (radians), translation xyz (metres).
    pub information_root: SMatrix<R, 6, 6>,
    pub huber_threshold: R,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> Factor<S> for AdjacentVisualOdometry<R> {
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        self.validate()?;
        let [a, b, c, d, e] = self.controls;
        let first = common::spline(states, &[a, b, c, d], self.duration)?;
        let second = common::spline(states, &[b, c, d, e], self.duration)?;
        let (error, _) = error(
            &first.pose(self.observation.previous_tau)?.inner,
            &second.pose(self.observation.current_tau)?.inner,
            &self.observation,
        )?;
        Ok(common::huber(&(self.information_root * error), self.huber_threshold)?.0)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        self.validate()?;
        let [a, b, c, d, e] = self.controls;
        let first = common::spline(states, &[a, b, c, d], self.duration)?;
        let second = common::spline(states, &[b, c, d, e], self.duration)?;
        let first = first.linearize()?.pose(self.observation.previous_tau)?;
        let second = second.linearize()?.pose(self.observation.current_tau)?;
        let (error, delta) = error(&first.pose.inner, &second.pose.inner, &self.observation)?;
        let (residual, ha, hb) =
            local_model(error, &delta, &self.information_root, self.huber_threshold)?;
        let jacobians = std::array::from_fn(|i| {
            let mut j = SMatrix::<R, 6, 6>::zeros();
            if i < 4 {
                j += ha * first.jacobians[i];
            }
            if i > 0 {
                j += hb * second.jacobians[i - 1];
            }
            j
        });
        common::emit(sink, &self.controls, &residual, &jacobians)
    }
}

impl<R: RealField + Copy> AdjacentVisualOdometry<R> {
    fn validate(&self) -> Result<(), EvaluationError> {
        common::unique(&self.controls)?;
        if R::one() + self.observation.current_tau <= self.observation.previous_tau {
            return Err(EvaluationError::InvalidEvaluation);
        }
        validate(&self.information_root, self.huber_threshold)
    }
}

fn validate<R: RealField + Copy>(
    root: &SMatrix<R, 6, 6>,
    threshold: R,
) -> Result<(), EvaluationError> {
    common::finite(root.iter())?;
    common::positive(threshold)?;
    Ok(())
}

fn error<R: RealField + Copy>(
    previous: &nalgebra::Isometry3<R>,
    current: &nalgebra::Isometry3<R>,
    observation: &VisualOdometryObservation<R>,
) -> Result<(SVector<R, 6>, PoseControl<R>), EvaluationError> {
    let measurement = &observation.current_to_previous.inner;
    common::finite(
        measurement
            .translation
            .vector
            .iter()
            .chain(measurement.rotation.coords.iter()),
    )?;
    let delta = previous.inverse() * current;
    let difference = measurement.inverse() * delta;
    common::rotation_log(&difference.rotation)?;
    let error = PoseControl {
        pose: linear_algebra::Pose3::wrap(difference),
    }
    .log();
    common::finite(error.iter())?;
    Ok((
        error,
        PoseControl {
            pose: linear_algebra::Pose3::wrap(delta),
        },
    ))
}

type OdometryModel<R> = (SVector<R, 6>, SMatrix<R, 6, 6>, SMatrix<R, 6, 6>);

fn local_model<R: RealField + Copy>(
    error: SVector<R, 6>,
    delta: &PoseControl<R>,
    root: &SMatrix<R, 6, 6>,
    threshold: R,
) -> Result<OdometryModel<R>, EvaluationError> {
    let residual = root * error;
    let scale = common::huber(&residual, threshold)?.1;
    let end = root * PoseControl::right_jacobian_inverse(&error) * scale;
    let start = -end * delta.inverse().adjoint();
    Ok((residual * scale, start, end))
}
