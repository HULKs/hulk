use color_eyre::Result;
use nalgebra::{DVector, Dyn, Owned, Vector};
use types::field_dimensions::FieldDimensions;

use crate::corrections::Corrections;

pub type Residual = Vector<f32, Dyn, ResidualStorage>;
pub type ResidualStorage = Owned<f32, Dyn>;

pub fn calculate_residuals_from_parameters<MeasurementType, ResidualType>(
    parameters: &Corrections,
    measurements: &[MeasurementType],
    field_dimensions: &FieldDimensions,
) -> Option<Residual>
where
    ResidualType: ResidualsCalculateFrom<MeasurementType>,
    Vec<f32>: From<ResidualType>,
{
    let mut residuals = Vec::new();
    for measurement in measurements {
        let residuals_part: Vec<f32> =
            ResidualType::calculate_from(parameters, measurement, field_dimensions)
                .ok()?
                .into();
        residuals.extend(residuals_part);
    }

    Some(DVector::from_vec(residuals))
}

pub trait ResidualsCalculateFrom<MeasurementType> {
    fn calculate_from(
        parameters: &Corrections,
        measurement: &MeasurementType,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self>
    where
        Self: Sized;
}
