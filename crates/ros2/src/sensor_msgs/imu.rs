/// This is a message to hold data from an IMU (Inertial Measurement Unit)
///
/// Accelerations should be in m/s^2 (not in g's), and rotational velocity should be in rad/sec
///
/// If the covariance of the measurement is known, it should be filled in (if all you know is the
/// variance of each measurement, e.g. from the datasheet, just put those along the diagonal)
/// A covariance matrix of all zeros will be interpreted as "covariance unknown", and to use the
/// data a covariance will have to be assumed or gotten from some other source
///
/// If you have no estimate for one of the data elements (e.g. your IMU doesn't produce an
/// orientation estimate), please set element 0 of the associated covariance matrix to -1
/// If you are interpreting this message, please check for a value of -1 in the first element of each
/// covariance matrix, and disregard the associated estimate.
use serde::{Deserialize, Serialize};

use crate::{
    geometry_msgs::{quaternion::Quaternion, vector3::Vector3},
    std_msgs::header::Header,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Imu {
    pub header: Header,

    pub orientation: Quaternion,
    /// Row major about x, y, z axes
    pub orientation_covariance: [f64; 9],

    pub angular_velocity: Vector3,
    /// Row major about x, y, z axes
    pub angular_velocity_covariance: [f64; 9],

    pub linear_acceleration: Vector3,
    /// Row major x, y z
    pub linear_acceleration_covariance: [f64; 9],
}
