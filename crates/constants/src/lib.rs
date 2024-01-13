use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct HardwareId {
    pub body_id: String,
    pub head_id: String,
}

pub const HULA_DBUS_INTERFACE: &str = "org.hulks.hula";
pub const HULA_DBUS_PATH: &str = "/org/hulks/HuLA";
pub const HULA_DBUS_SERVICE: &str = "org.hulks.hula";
pub const HULA_SOCKET_PATH: &str = "/tmp/hula";
pub const OS_RELEASE_PATH: &str = "/etc/os-release";
pub const OS_VERSION: &str = "5.9.1";
pub const SDK_VERSION: &str = "5.9.0";
lazy_static! {
    pub static ref HARDWARE_IDS: HashMap<u8, HardwareId> = {
        let content = include_str!("../../../etc/parameters/hardware_ids.json");
        serde_json::from_str(content).unwrap()
    };
}
