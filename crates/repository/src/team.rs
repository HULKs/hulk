use std::path::Path;

use color_eyre::{eyre::Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;

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

pub async fn get_team_configuration(repository_root: impl AsRef<Path>) -> Result<Team> {
    let team_toml = repository_root.as_ref().join("etc/parameters/team.toml");

    let content = read_to_string(&team_toml)
        .await
        .wrap_err_with(|| format!("failed to open {}", team_toml.display()))?;

    let team = toml::from_str(&content).wrap_err("failed to parse team.toml")?;
    Ok(team)
}
