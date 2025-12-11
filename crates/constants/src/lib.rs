use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

pub const OS_IS_NOT_LINUX: bool = !cfg!(target_os = "linux");
pub const OS_RELEASE_PATH: &str = "/etc/os-release";

#[derive(Serialize, Deserialize)]
pub struct Team {
    pub team_number: u8,
    pub naos: Vec<Nao>,
}

#[derive(Serialize, Deserialize)]
pub struct Nao {
    pub number: u8,
    pub hostname: String,
    pub body_id: String,
    pub head_id: String,
}

lazy_static! {
    pub static ref TEAM: Team = {
        let content = include_str!("../../../etc/parameters/team.toml");
        toml::from_str(content).unwrap()
    };
}
