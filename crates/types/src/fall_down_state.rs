use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[repr(u32)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(frozen, get_all))]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    ros_z::Message,
)]
pub enum FallDownStateType {
    IsReady = 0,
    IsFalling = 1,
    HasFallen = 2,
    IsGettingUp = 3,
}

#[repr(C)]
#[cfg_attr(feature = "pyo3", pyo3::pyclass(frozen, get_all))]
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    ros_z::Message,
)]
pub struct FallDownState {
    pub fall_down_state: FallDownStateType,
    pub is_recovery_available: bool,
}

#[cfg(feature = "pyo3")]
#[pyo3::pymethods]
impl FallDownState {
    #[new]
    pub fn new(fall_down_state: FallDownStateType, is_recovery_available: bool) -> Self {
        Self {
            fall_down_state,
            is_recovery_available,
        }
    }
}
