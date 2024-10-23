use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

pub const HULA_DBUS_INTERFACE: &str = "org.hulks.hula";
pub const HULA_DBUS_PATH: &str = "/org/hulks/HuLA";
pub const HULA_DBUS_SERVICE: &str = "org.hulks.hula";
pub const HULA_SOCKET_PATH: &str = "/tmp/hula";
pub const OS_IS_NOT_LINUX: bool = !cfg!(target_os = "linux");
pub const OS_RELEASE_PATH: &str = "/etc/os-release";
pub const OS_VERSION: &str = "7.5.7";
pub const SDK_VERSION: &str = "7.5.0";

#[derive(Serialize, Deserialize)]
pub struct Team {
    pub team_number: u8,
    pub hostname_prefix: String,
    pub naos: Vec<Nao>,
}

#[derive(Serialize, Deserialize)]
pub struct Nao {
    pub number: u8,
    pub body_id: String,
    pub head_id: String,
}

lazy_static! {
    pub static ref TEAM: Team = {
        let content = include_str!("../../../etc/parameters/team.toml");
        toml::from_str(content).unwrap()
    };
}
