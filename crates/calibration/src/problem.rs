use std::sync::Mutex;

use levenberg_marquardt::LeastSquaresProblem;
use nalgebra::{Const, Dyn, Owned, SVector};
use types::FieldDimensions;

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
    metrics: Mutex<Vec<Metric>>,
}

#[derive(Clone, Debug)]
pub enum Metric {
    ParametersSet { parameters: Corrections },
    ResidualsGet { residuals: Option<Residual> },
}

impl CalibrationProblem {
    pub fn new(
        initial_corrections: Corrections,
        measurements: Vec<Measurement>,
        field_dimensions: FieldDimensions,
    ) -> Self {
        println!("CalibrationProblem::new({initial_corrections:?}, {measurements:?}, {field_dimensions:?})");
        Self {
            parameters: initial_corrections,
            measurements,
            field_dimensions,
            metrics: Mutex::new(Vec::new()),
        }
    }

    pub fn get_corrections(&self) -> Corrections {
        self.parameters
    }

    pub fn get_metrics(&self) -> Vec<Metric> {
        self.metrics.lock().unwrap().clone()
    }
}

impl LeastSquaresProblem<f32, Dyn, Const<AMOUNT_OF_PARAMETERS>> for CalibrationProblem {
    type ResidualStorage = ResidualStorage;
    type JacobianStorage = JacobianStorage;
    type ParameterStorage = Owned<f32, Const<AMOUNT_OF_PARAMETERS>>;

    fn set_params(&mut self, parameters: &SVector<f32, AMOUNT_OF_PARAMETERS>) {
        // println!("set_params({parameters:?})");
        self.parameters = parameters.into();
        self.metrics.lock().unwrap().push(Metric::ParametersSet {
            parameters: self.parameters.clone(),
        });
    }

    fn params(&self) -> SVector<f32, AMOUNT_OF_PARAMETERS> {
        let result = (&self.parameters).into();
        // println!("params() -> {result:?}");
        result
    }

    fn residuals(&self) -> Option<Residual> {
        let result = calculate_residuals_from_parameters(
            &self.parameters,
            &self.measurements,
            &self.field_dimensions,
        );
        // println!("residuals() -> {result:?}");
        self.metrics.lock().unwrap().push(Metric::ResidualsGet {
            residuals: result.clone(),
        });
        result
    }

    fn jacobian(&self) -> Option<Jacobian> {
        let result = calculate_jacobian_from_parameters(
            &self.parameters,
            &self.measurements,
            &self.field_dimensions,
        );
        // println!("jacobian() -> {result:?}");
        result
    }
}
