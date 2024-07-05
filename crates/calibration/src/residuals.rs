use color_eyre::Result;
use nalgebra::{DVector, Dyn, Owned, Vector};
use types::field_dimensions::FieldDimensions;

use crate::corrections::Corrections;

pub type ResidualVector = Vector<f32, Dyn, ResidualVectorStorage>;
pub type ResidualVectorStorage = Owned<f32, Dyn>;

pub fn calculate_residuals_from_parameters<MeasurementType, ResidualsFromMeasurement>(
    parameters: &Corrections,
    measurements: &[MeasurementType],
    field_dimensions: &FieldDimensions,
) -> Option<ResidualVector>
where
    ResidualsFromMeasurement: CalculateResiduals<MeasurementType>,
    Vec<f32>: From<ResidualsFromMeasurement>,
{
    let mut residuals = Vec::new();
    for measurement in measurements {
        let residuals_part: Vec<f32> =
            ResidualsFromMeasurement::calculate_from(parameters, measurement, field_dimensions)
                .ok()?
                .into();
        residuals.extend(residuals_part);
    }

    Some(DVector::from_vec(residuals))
}

pub trait CalculateResiduals<MeasurementType> {
    fn calculate_from(
        parameters: &Corrections,
        measurement: &MeasurementType,
        field_dimensions: &FieldDimensions,
    ) -> Result<Self>
    where
        Self: Sized;
}
