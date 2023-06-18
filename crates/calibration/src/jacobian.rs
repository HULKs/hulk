use nalgebra::{Const, Dyn, Matrix, Owned, SVector};
use types::FieldDimensions;

use crate::{
    corrections::{Corrections, AMOUNT_OF_PARAMETERS},
    measurement::Measurement,
    residuals::calculate_residuals_from_parameters,
};

pub type Jacobian = Matrix<f32, Dyn, Const<AMOUNT_OF_PARAMETERS>, JacobianStorage>;
pub type JacobianStorage = Owned<f32, Dyn, Const<AMOUNT_OF_PARAMETERS>>;

const EPSILON: f32 = 0.000001;

pub fn calculate_jacobian_from_parameters(
    parameters: &Corrections,
    measurements: &[Measurement],
    field_dimensions: &FieldDimensions,
) -> Option<Jacobian> {
    let columns = (0..AMOUNT_OF_PARAMETERS)
        .map(|index| {
            let (upper_support_parameters, lower_support_parameters) =
                get_parameter_support_points_from_parameters_with_epsilon(
                    parameters, index, EPSILON,
                );
            Some(
                (calculate_residuals_from_parameters(
                    &upper_support_parameters,
                    measurements,
                    field_dimensions,
                )? - calculate_residuals_from_parameters(
                    &lower_support_parameters,
                    measurements,
                    field_dimensions,
                )?) / (2.0 * EPSILON),
            )
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Matrix::from_columns(&columns))
}

fn get_parameter_support_points_from_parameters_with_epsilon(
    parameters: &Corrections,
    epsilon_index: usize,
    epsilon: f32,
) -> (Corrections, Corrections) {
    let parameters: SVector<f32, AMOUNT_OF_PARAMETERS> = parameters.into();
    let epsilon_vector = SVector::<f32, AMOUNT_OF_PARAMETERS>::from_vec(
        (0..AMOUNT_OF_PARAMETERS)
            .map(|index| if index == epsilon_index { epsilon } else { 0.0 })
            .collect(),
    );
    (
        (&(parameters + epsilon_vector)).into(),
        (&(parameters - epsilon_vector)).into(),
    )
}
