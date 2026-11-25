use std::{
    ffi::OsString,
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::Result;
use tokio::process::Command;

use crate::{Repository, sdk};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Environment {
    PathCargo,
    #[value(alias = "native")]
    NixNative,
    #[value(alias = "nix", alias = "auto")]
    NixRobot,
}

pub const ROBOT_TARGET_DIRECTORY: &str = "target/robot";

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathCargo => write!(f, "Cargo on PATH"),
            Self::NixNative => write!(f, "Workstation Nix shell"),
            Self::NixRobot => write!(f, "Nix robot SDK"),
        }
    }
}

#[derive(Debug)]
pub enum Host {
    Local,
    Remote,
}

#[derive(Debug)]
pub struct Cargo {
    host: Host,
    environment: Environment,
    arguments: Vec<OsString>,
}

impl Cargo {
    pub fn local(environment: Environment) -> Self {
        Self {
            host: Host::Local,
            environment,
            arguments: Vec::new(),
        }
    }

    pub fn remote(environment: Environment) -> Self {
        Self {
            host: Host::Remote,
            environment,
            arguments: Vec::new(),
        }
    }

    pub fn arg(&mut self, argument: impl Into<OsString>) -> &mut Self {
        self.arguments.push(argument.into());
        self
    }

    pub fn args(&mut self, arguments: impl IntoIterator<Item = impl Into<OsString>>) -> &mut Self {
        self.arguments.extend(arguments.into_iter().map(Into::into));
        self
    }

    pub fn command(
        self,
        repository: &Repository,
        compiler_artifacts: &[impl AsRef<Path>],
    ) -> Result<Command> {
        let root = match self.host {
            Host::Local => repository.root.clone(),
            Host::Remote => repository.current_dir_to_root()?,
        };
        let mut command = match self.environment {
            Environment::PathCargo => Command::new("cargo"),
            Environment::NixNative => sdk::native_cargo_command(&root),
            Environment::NixRobot => {
                let mut command = sdk::robot_command(&root);
                command.arg("cargo");
                command
            }
        };
        command.args(self.arguments);

        if matches!(self.host, Host::Local) {
            return Ok(command);
        }

        let mut remote = Command::new(repository.root.join("scripts/remote_workspace"));
        for path in compiler_artifacts {
            remote.arg("--return-file").arg(path.as_ref());
        }
        remote.arg("--cd").arg(repository.root_to_current_dir()?);
        let command = command.as_std();
        let arguments = std::iter::once(command.get_program()).chain(command.get_args());
        for argument in arguments {
            remote.arg(argument);
        }
        Ok(remote)
    }
}
