pub mod builtin_interfaces;
pub mod geometry_msgs;
pub mod sensor_msgs;
pub mod std_msgs;

#[cfg(feature = "pyo3")]
use pyo3::pymodule;

#[cfg(feature = "pyo3")]
#[pymodule(name = "ros2_types")]
pub mod python_module {
    #[pymodule_export]
    use crate::sensor_msgs::camera_info::CameraInfo;
    #[pymodule_export]
    use crate::sensor_msgs::image::Image;
}
