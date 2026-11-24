use std::str::FromStr;

use clap::Args;
use color_eyre::eyre::{Error, Result, bail};
use repository::cargo::Environment as RepositoryEnvironment;

#[derive(Args, Debug, Clone)]
pub struct EnvironmentArguments {
    /// Use the workstation Nix shell (native) or robot SDK (nix, alias: auto)
    #[arg(long, visible_alias = "umgebung", visible_alias = "umg")]
    pub env: Option<Environment>,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long, visible_alias = "fern", visible_alias = "fernbedienung")]
    pub remote: bool,
}

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Robot,
}

impl FromStr for Environment {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self> {
        match string {
            "native" | "ureinwohner" | "einheimisch" => Ok(Self::Native),
            "nix" | "auto" => Ok(Self::Robot),
            _ => bail!("unknown environment {string}"),
        }
    }
}

impl Environment {
    pub fn resolve(self) -> RepositoryEnvironment {
        match self {
            Self::Native => RepositoryEnvironment::Native,
            Self::Robot => RepositoryEnvironment::Robot,
        }
    }
}
