use factrs::{
    containers::Key,
    linalg::{DiffResult, MatrixX, VectorX},
    residuals::{DynResidual, DynValues, ResidualError},
    variables::VariableSafe,
};

#[derive(Debug, Clone)]
pub struct SchurPriorResidual {
    keys: Vec<Key>,
    jacobian: MatrixX,
    target: VectorX,
    linearization_point: Vec<Box<dyn VariableSafe>>,
}

impl SchurPriorResidual {
    pub fn new(
        keys: Vec<Key>,
        jacobian: MatrixX,
        target: VectorX,
        linearization_point: Vec<Box<dyn VariableSafe>>,
    ) -> Self {
        Self {
            keys,
            jacobian,
            target,
            linearization_point,
        }
    }
}

#[factrs::mark]
impl DynResidual for SchurPriorResidual {
    fn dim_out(&self, keys: &[Key]) -> Result<usize, ResidualError> {
        assert_eq!(keys, &self.keys);
        Ok(self.jacobian.nrows())
    }

    fn residual(&self, input: &DynValues) -> VectorX {
        let keys = input.keys();
        assert_eq!(keys, &self.keys);

        let mut dx = VectorX::zeros(self.jacobian.ncols());
        let mut offset = 0;
        for (key, linear) in keys.iter().zip(self.linearization_point.iter()) {
            let current = input.get_raw(*key).expect("key is missing");
            let delta = current.ominus_safe(linear.as_ref());
            dx.rows_mut(offset, delta.len()).copy_from(&delta);
            offset += delta.len();
        }

        &self.jacobian * dx - &self.target
    }

    fn residual_jacobian(&self, input: &DynValues) -> DiffResult<VectorX, MatrixX> {
        let value = self.residual(input);
        // We use the Jacobian as the derivative of the residual with respect to the linearization point here.
        // This is not entirely correct, as the influence of the current values on the residual is not taken into account.
        // However, if the correct derivative were used, this would introduce incorrect information about singular values (see First Estimate Jacobian).
        DiffResult {
            value,
            diff: self.jacobian.clone(),
        }
    }
}
