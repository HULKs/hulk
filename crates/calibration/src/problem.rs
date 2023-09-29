use levenberg_marquardt::LeastSquaresProblem;
use nalgebra::{Const, Dyn, Owned, SVector};
use types::field_dimensions::FieldDimensions;

use crate::{
    corrections::{Corrections, AMOUNT_OF_PARAMETERS},
    jacobian::{calculate_jacobian_from_parameters, Jacobian, JacobianStorage},
    measurement::Measurement,
    residuals::{calculate_residuals_from_parameters, Residual, ResidualStorage},
};

pub struct CalibrationProblem {
    parameters: Corrections,
    measurements: Vec<Measurement>,
    field_dimensions: FieldDimensions,
}

impl CalibrationProblem {
    pub fn new(
        initial_corrections: Corrections,
        measurements: Vec<Measurement>,
        field_dimensions: FieldDimensions,
    ) -> Self {
        Self {
            parameters: initial_corrections,
            measurements,
            field_dimensions,
        }
    }

    pub fn get_corrections(&self) -> Corrections {
        self.parameters
    }
}

impl LeastSquaresProblem<f32, Dyn, Const<AMOUNT_OF_PARAMETERS>> for CalibrationProblem {
    type ResidualStorage = ResidualStorage;
    type JacobianStorage = JacobianStorage;
    type ParameterStorage = Owned<f32, Const<AMOUNT_OF_PARAMETERS>>;

    fn set_params(&mut self, parameters: &SVector<f32, AMOUNT_OF_PARAMETERS>) {
        println!("set_params({parameters:?})");
        self.parameters = parameters.into();
    }

    fn params(&self) -> SVector<f32, AMOUNT_OF_PARAMETERS> {
        println!("params()");
        (&self.parameters).into()
    }

    fn residuals(&self) -> Option<Residual> {
        println!("residuals()");
        calculate_residuals_from_parameters(
            &self.parameters,
            &self.measurements,
            &self.field_dimensions,
        )
    }

    fn jacobian(&self) -> Option<Jacobian> {
        println!("jacobian()");
        calculate_jacobian_from_parameters(
            &self.parameters,
            &self.measurements,
            &self.field_dimensions,
        )
    }
}
