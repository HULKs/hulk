use std::{collections::HashMap, str::FromStr};

use chrono::Utc;
use color_eyre::{eyre::WrapErr, Result};
use serde::{de::Error as DeserializeError, Deserialize, Deserializer};
use spl_network_messages::PlayerNumber;
use tokio::fs::read_to_string;
use toml::from_str;

use argument_parsers::{
    parse_network, NaoAddress, NaoAddressPlayerAssignment, NaoNumberPlayerAssignment,
};
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
    #[serde(deserialize_with = "deserialize_assignments")]
    pub substitutions: Vec<NaoAddressPlayerAssignment>,
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

    pub fn log_directory_name(&self) -> String {
        let date = Utc::now().date_naive();

        if let Some(opponent) = &self.opponent {
            format!("{date}-HULKs-vs-{opponent}")
        } else {
            format!("{date}-testgame")
        }
    }

    pub fn branch_name(&self) -> String {
        let mut branch_name = self.log_directory_name();

        if let Some(phase) = &self.phase {
            branch_name.push('-');
            branch_name.push_str(phase);
        }

        branch_name
    }

    pub fn playing_naos(&self) -> Vec<NaoAddress> {
        self.assignments().into_values().collect()
    }

    pub fn all_naos(&self) -> Vec<NaoAddress> {
        let mut naos: Vec<_> = self
            .assignments
            .iter()
            .chain(&self.substitutions)
            .map(|assignment| assignment.nao_address)
            .collect();

        naos.sort_unstable();
        naos.dedup();

        naos
    }

    pub async fn configure_repository(self, repository: &Repository) -> Result<()> {
        player_number(
            Arguments {
                assignments: self
                    .assignments()
                    .into_iter()
                    .map(|(player_number, nao_address)| {
                        nao_address
                            .try_into()
                            .map(|nao_number| NaoNumberPlayerAssignment {
                                nao_number,
                                player_number,
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .wrap_err("failed to convert NAO addresses to NAO numbers")?,
            },
            repository,
        )
        .await
        .wrap_err("failed to set player numbers")?;

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

        Ok(())
    }

    fn assignments(&self) -> HashMap<PlayerNumber, NaoAddress> {
        let mut assignments: HashMap<_, _> = self
            .assignments
            .iter()
            .map(|assignment| (assignment.player_number, assignment.nao_address))
            .collect();

        for substitution in &self.substitutions {
            assignments.insert(substitution.player_number, substitution.nao_address);
        }

        assignments
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
