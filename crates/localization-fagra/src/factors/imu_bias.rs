use super::common;
use crate::variables::ImuBias;
use fagra::{
    BlockId, EvaluationError, Factor, JacobianBlock, LinearizationSink, StateKey, StateStore,
    Variable,
};
use nalgebra::{RealField, SMatrix};

/// Independent calibration prior; never substitute a replayed measurement posterior.
#[derive(Clone, Debug)]
pub struct ImuBiasPrior<R: RealField + Copy = f64> {
    pub bias: StateKey<ImuBias<R>>,
    pub reference: ImuBias<R>,
    pub information_root: SMatrix<R, 6, 6>,
}

impl<R: RealField + Copy, S: StateStore<ImuBias<R>>> Factor<S> for ImuBiasPrior<R> {
    type Scalar = R;
    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        visitor(self.bias.block_id());
    }
    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::finite(self.information_root.iter())?;
        common::cost(
            &(self.information_root * (states.get(self.bias)?.log() - self.reference.log())),
        )
    }
    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::finite(self.information_root.iter())?;
        let residual =
            self.information_root * (states.get(self.bias)?.log() - self.reference.log());
        sink.residual(
            &residual,
            &[JacobianBlock::new(self.bias, &self.information_root)],
        )
    }
}

/// Random walk between coarse bias knots. Root includes 1/sqrt(knot duration).
#[derive(Clone, Debug)]
pub struct ImuBiasWalk<R: RealField + Copy = f64> {
    pub biases: [StateKey<ImuBias<R>>; 2],
    pub information_root: SMatrix<R, 6, 6>,
}

impl<R: RealField + Copy, S: StateStore<ImuBias<R>>> Factor<S> for ImuBiasWalk<R> {
    type Scalar = R;
    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.biases {
            visitor(key.block_id());
        }
    }
    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::finite(self.information_root.iter())?;
        common::cost(
            &(self.information_root
                * (states.get(self.biases[1])?.log() - states.get(self.biases[0])?.log())),
        )
    }
    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::finite(self.information_root.iter())?;
        let residual = self.information_root
            * (states.get(self.biases[1])?.log() - states.get(self.biases[0])?.log());
        sink.residual(
            &residual,
            &[
                JacobianBlock::new(self.biases[0], &-self.information_root),
                JacobianBlock::new(self.biases[1], &self.information_root),
            ],
        )
    }
}
