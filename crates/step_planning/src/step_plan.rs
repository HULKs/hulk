use nalgebra::RealField;

use crate::{geometry::normalized_step::NormalizedStep, VARIABLES_PER_STEP};

pub struct StepPlan<'a, T>(&'a [T]);

impl<'a, T> From<&'a [T]> for StepPlan<'a, T> {
    fn from(value: &'a [T]) -> Self {
        assert!(value.len() % VARIABLES_PER_STEP == 0);

        Self(value)
    }
}

impl<'a, T: RealField> StepPlan<'a, T> {
    pub fn steps(&self) -> impl Iterator<Item = NormalizedStep<T>> + 'a {
        self.0.chunks_exact(3).map(NormalizedStep::from_slice)
    }
}
