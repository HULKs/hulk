#[cfg(feature = "pyo3")]
use pyo3::types::PyModuleMethods;

pub use ros_z_msgs::{builtin_interfaces, geometry_msgs, sensor_msgs, std_msgs};

#[cfg(feature = "pyo3")]
pub mod pyo3_compat {
    use std::time::Duration;

    #[pyo3::pyclass(frozen, get_all)]
    #[derive(Clone, Debug)]
    pub struct PyTime {
        pub sec: i32,
        pub nanosec: u32,
    }

    #[pyo3::pyclass(frozen, get_all)]
    #[derive(Clone, Debug)]
    pub struct PyHeader {
        pub stamp: PyTime,
        pub frame_id: String,
    }

    #[pyo3::pyclass(frozen, get_all)]
    #[derive(Clone, Debug)]
    pub struct PyRegionOfInterest {
        pub x_offset: u32,
        pub y_offset: u32,
        pub height: u32,
        pub width: u32,
        pub do_rectify: bool,
    }

    pub mod sensor_msgs {
        use super::{Duration, PyHeader, PyRegionOfInterest, PyTime};

        #[pyo3::pyclass(frozen, get_all)]
        #[derive(Clone, Debug)]
        pub struct PyImage {
            pub header: PyHeader,
            pub height: u32,
            pub width: u32,
            pub encoding: String,
            pub is_bigendian: u8,
            pub step: u32,
            pub data: Vec<u8>,
        }

        #[pyo3::pymethods]
        impl PyImage {
            #[new]
            pub fn new(time: f32, rgb: Vec<u8>, height: u32, width: u32) -> Self {
                let simulation_duration = Duration::from_secs_f32(time);

                Self {
                    header: PyHeader {
                        stamp: PyTime {
                            sec: simulation_duration.as_secs() as i32,
                            nanosec: simulation_duration.subsec_nanos(),
                        },
                        frame_id: String::new(),
                    },
                    height,
                    width,
                    encoding: "rgb8".to_string(),
                    is_bigendian: 0,
                    step: width.saturating_mul(3),
                    data: rgb,
                }
            }
        }

        #[pyo3::pyclass(frozen, get_all)]
        #[derive(Clone, Debug)]
        pub struct PyCameraInfo {
            pub header: PyHeader,
            pub height: u32,
            pub width: u32,
            pub distortion_model: String,
            pub d: Vec<f64>,
            pub k: [f64; 9],
            pub r: [f64; 9],
            pub p: [f64; 12],
            pub binning_x: u32,
            pub binning_y: u32,
            pub roi: PyRegionOfInterest,
        }

        #[pyo3::pymethods]
        impl PyCameraInfo {
            #[new]
            pub fn new(
                time: f32,
                height: u32,
                width: u32,
                focal_length_x: f32,
                focal_length_y: f32,
                optical_center_x: f32,
                optical_center_y: f32,
            ) -> Self {
                let simulation_duration = Duration::from_secs_f32(time);

                let (fx, fy, cx, cy) = (
                    focal_length_x as f64,
                    focal_length_y as f64,
                    optical_center_x as f64,
                    optical_center_y as f64,
                );

                #[rustfmt::skip]
                let k = [
                    fx, 0.0, cx,
                    0.0, fy, cy,
                    0.0, 0.0, 1.0,
                ];

                #[rustfmt::skip]
                let p = [
                    fx, 0.0, cx, 0.0,
                    0.0, fy, cy, 0.0,
                    0.0, 0.0, 1.0, 0.0,
                ];

                Self {
                    header: PyHeader {
                        stamp: PyTime {
                            sec: simulation_duration.as_secs() as i32,
                            nanosec: simulation_duration.subsec_nanos(),
                        },
                        frame_id: String::new(),
                    },
                    height,
                    width,
                    distortion_model: String::new(),
                    d: Vec::new(),
                    k,
                    r: Default::default(),
                    p,
                    binning_x: 0,
                    binning_y: 0,
                    roi: PyRegionOfInterest {
                        x_offset: 0,
                        y_offset: 0,
                        height: 0,
                        width: 0,
                        do_rectify: false,
                    },
                }
            }
        }

        pub mod camera_info {
            pub use super::PyCameraInfo as CameraInfo;
        }

        pub mod image {
            pub use super::PyImage as Image;
        }

        pub mod region_of_interest {
            pub use super::super::PyRegionOfInterest as RegionOfInterest;
        }
    }

    pub mod std_msgs {
        pub mod header {
            pub use super::super::PyHeader as Header;
        }
    }

    pub mod builtin_interfaces {
        pub mod time {
            pub use super::super::PyTime as Time;
        }
    }
}

#[cfg(feature = "pyo3")]
#[pyo3::pymodule(name = "ros2_types")]
pub fn python_module(
    py: pyo3::Python<'_>,
    module: &pyo3::Bound<'_, pyo3::types::PyModule>,
) -> pyo3::PyResult<()> {
    module.add(
        "Image",
        py.get_type::<crate::pyo3_compat::sensor_msgs::PyImage>(),
    )?;
    module.add(
        "CameraInfo",
        py.get_type::<crate::pyo3_compat::sensor_msgs::PyCameraInfo>(),
    )?;
    Ok(())
}
