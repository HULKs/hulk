use std::{
    ffi::OsString,
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::Result;
use tokio::process::Command;

use crate::{Repository, sdk};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Robot,
}

pub const ROBOT_TARGET_DIRECTORY: &str = "target/robot";

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "Native"),
            Self::Robot => write!(f, "Robot SDK"),
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
            Environment::Native => sdk::native_cargo_command(&root),
            Environment::Robot => {
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
