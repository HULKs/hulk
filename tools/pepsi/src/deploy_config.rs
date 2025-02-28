use std::{collections::HashMap, str::FromStr};

use chrono::Utc;
use color_eyre::{eyre::WrapErr, Result};
use serde::{de::Error as DeserializeError, Deserialize, Deserializer};
use tokio::fs::read_to_string;
use toml::from_str;

use argument_parsers::{parse_network, NaoAddress, NaoAddressPlayerAssignment};
use nao::Network;
use repository::Repository;

use crate::player_number::{player_number, Arguments};

#[derive(Deserialize)]
pub struct DeployConfig {
    pub opponent: Option<String>,
    pub phase: Option<String>,
    pub location: String,
    #[serde(deserialize_with = "deserialize_network")]
    pub wifi: Network,
    pub base: String,
    pub branches: Vec<Branch>,
    #[serde(deserialize_with = "deserialize_assignments")]
    pub assignments: Vec<NaoAddressPlayerAssignment>,
    pub with_communication: bool,
    pub recording_intervals: HashMap<String, usize>,
}

impl DeployConfig {
    pub async fn read_from_file(repository: &Repository) -> Result<Self> {
        let deploy_config = read_to_string(repository.root.join("deploy.toml"))
            .await
            .wrap_err("failed to read deploy.toml")?;

        from_str(&deploy_config).wrap_err("could not deserialize config from deploy.toml")
    }

    pub fn branch_name(&self) -> String {
        let date = Utc::now().date_naive();

        let mut branch_name = if let Some(opponent) = &self.opponent {
            format!("{date}-HULKs-vs-{opponent}")
        } else {
            format!("{date}-testgame")
        };

        if let Some(phase) = &self.phase {
            branch_name.push('-');
            branch_name.push_str(phase);
        }

        branch_name
    }

    pub fn naos(&self) -> Vec<NaoAddress> {
        self.assignments
            .iter()
            .map(|assignment| assignment.nao_address)
            .collect()
    }

    pub async fn configure_repository(self, repository: &Repository) -> Result<()> {
        repository
            .configure_recording_intervals(self.recording_intervals)
            .await
            .wrap_err("failed to apply recording settings")?;

        repository
            .set_location("nao", &self.location)
            .await
            .wrap_err_with(|| format!("failed to set location for nao to {}", self.location))?;

        repository
            .configure_communication(self.with_communication)
            .await
            .wrap_err("failed to set communication")?;

        player_number(
            Arguments {
                assignments: self
                    .assignments
                    .iter()
                    .copied()
                    .map(TryFrom::try_from)
                    .collect::<Result<Vec<_>, _>>()?,
            },
            repository,
        )
        .await
        .wrap_err("failed to set player numbers")?;

        Ok(())
    }
}

fn deserialize_network<'de, D, E>(deserializer: D) -> Result<Network, E>
where
    D: Deserializer<'de>,
    E: DeserializeError + From<D::Error>,
{
    let network = String::deserialize(deserializer)?;

    parse_network(&network).map_err(|error| E::custom(format!("{error:?}")))
}

fn deserialize_assignments<'de, D, E>(deserializer: D) -> Result<Vec<NaoAddressPlayerAssignment>, E>
where
    D: Deserializer<'de>,
    E: DeserializeError + From<D::Error>,
{
    let assignments: Vec<String> = Vec::deserialize(deserializer)?;

    assignments
        .into_iter()
        .map(|assignment| {
            NaoAddressPlayerAssignment::from_str(&assignment)
                .map_err(|error| E::custom(format!("{error:?}")))
        })
        .collect()
}

pub struct Branch {
    pub remote: String,
    pub branch: String,
}

impl<'de> Deserialize<'de> for Branch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let branch = String::deserialize(deserializer)?;

        let (remote, branch) = branch.split_once("/").ok_or_else(|| {
            D::Error::custom("deploy target has to follow the format 'remote/branch'")
        })?;

        Ok(Self {
            remote: remote.to_owned(),
            branch: branch.to_owned(),
        })
    }
}
