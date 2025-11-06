/// Measurement of the Magnetic Field vector at a specific location.
///
/// If the covariance of the measurement is known, it should be filled in.
/// If all you know is the variance of each measurement, e.g. from the datasheet,
/// just put those along the diagonal.
/// A covariance matrix of all zeros will be interpreted as "covariance unknown",
/// and to use the data a covariance will have to be assumed or gotten from some
/// other source.
use serde::{Deserialize, Serialize};

use crate::{geometry_msgs::vector3::Vector3, std_msgs::header::Header};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MagneticField {
    /// timestamp is the time the
    /// frame_id is the location and orientation
    /// of the field measurement
    pub header: Header,
    /// field was measured
    /// x, y, and z components of the
    pub magnetic_field: Vector3,
    /// field vector in Tesla
    /// If your sensor does not output 3 axes,
    /// put NaNs in the components not reported.
    /// Row major about x, y, z axes
    pub magnetic_field_covariance: [f64; 9],
}

impl MagneticField {
    pub fn default_with_header(header: Header) -> Self {
        Self {
            header,
            magnetic_field: Vector3::default(),
            magnetic_field_covariance: [0.0; 9],
        }
    }
}
