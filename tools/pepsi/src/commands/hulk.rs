use std::{net::Ipv4Addr, path::PathBuf, str::FromStr};

use anyhow::Context;
use log::info;

use crate::naossh;

#[derive(Clone, Copy, Debug)]
pub enum Command {
    Stop,
    Start,
    Restart,
    Enable,
    Disable,
}

impl Command {
    fn to_service_call(self) -> &'static str {
        match self {
            Command::Stop => "hulk stop",
            Command::Start => "hulk start",
            Command::Restart => "hulk restart",
            Command::Enable => "hulk enable",
            Command::Disable => "hulk disable",
        }
    }
}

impl FromStr for Command {
    type Err = anyhow::Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "stop" => Ok(Command::Stop),
            "start" => Ok(Command::Start),
            "restart" => Ok(Command::Restart),
            "enable" => Ok(Command::Enable),
            "disable" => Ok(Command::Disable),
            _ => Err(anyhow::anyhow!("cannot parse Command from str")),
        }
    }
}

pub async fn hulk_service(
    nao: Ipv4Addr,
    command: Command,
    project_root: PathBuf,
) -> anyhow::Result<String> {
    let command = command.to_service_call();
    info!("Sending '{}' to {}", command, nao);
    let output = naossh::command(nao, command, &project_root)
        .await
        .with_context(|| format!("HULK service call '{}' on {} failed", command, nao))?;
    if output.exit_status != Some(0) {
        anyhow::bail!(
            "HULK service call on {} failed with {:?}",
            nao,
            output.exit_status
        )
    }
    info!("Successful '{}' on {}", command, nao);
    Ok(output.stdout)
}
