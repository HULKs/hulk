use std::net::Ipv4Addr;

pub use hula_types::Battery;
use serde::{Deserialize, Serialize};
use service_manager::SystemServices;

pub mod service_manager;

pub const BEACON_MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 42);
pub const BEACON_PORT: u16 = 4242;
pub const BEACON_HEADER: &[u8; 6] = b"BEACON";

#[derive(Debug, Serialize)]
pub struct BeaconResponse<'a> {
    pub hostname: &'a str,
    pub interface_name: &'a str,
    pub system_services: &'a SystemServices,
    pub hulks_os_version: &'a str,
    pub body_id: &'a Option<String>,
    pub head_id: &'a Option<String>,
    pub battery: &'a Option<Battery>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlivenessState {
    pub hostname: String,
    pub interface_name: String,
    pub system_services: SystemServices,
    pub hulks_os_version: String,
    pub body_id: Option<String>,
    pub head_id: Option<String>,
    pub battery: Option<Battery>,
}
