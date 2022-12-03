use std::ffi::OsString;

use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum ActiveState {
    Activating,
    Active,
    Deactivating,
    Failed,
    Inactive,
    NotLoaded,
    Reloading,
    Unknown,
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct SystemServices {
    pub hal_state: ActiveState,
    pub hula_state: ActiveState,
    pub hulk_state: ActiveState,
    pub lola_state: ActiveState,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BeaconResponse {
    pub hostname: OsString,
    pub interface_name: String,
    pub system_services: SystemServices,
    pub hulks_os_version: String,
    pub body_id: String,
    pub head_id: String,
    pub battery_charge: f32,
    pub battery_current: f32,
}
