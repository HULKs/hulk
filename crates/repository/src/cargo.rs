use core::fmt;
use std::{
    ffi::{OsStr, OsString},
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use tokio::process::Command;

use crate::{sdk::download_and_install, Repository};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Sdk { version: String },
    Docker { image: String },
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Native => write!(f, "Native"),
            Environment::Sdk { version } => write!(f, "SDK ({version})"),
            Environment::Docker { image } => write!(f, "Docker ({image})"),
        }
    }
}

pub enum Host {
    Local,
    Remote,
}

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

    pub async fn setup(&self, repository: &Repository) -> Result<()> {
        if let Environment::Sdk { version } = &self.environment {
            match self.host {
                Host::Local => {
                    let data_home = repository
                        .resolve_data_home()
                        .await
                        .wrap_err("failed to resolve data home")?;

                    download_and_install(version, data_home)
                        .await
                        .wrap_err("failed to download and install SDK")?;
                }
                Host::Remote => {
                    let mut command =
                        Command::new(repository.root.join("scripts/remote_workspace"));

                    let status = command
                        .arg("pepsi")
                        .arg("sdk")
                        .arg("install")
                        .arg("--version")
                        .arg(version)
                        .status()
                        .await
                        .wrap_err("failed to run pepsi")?;

                    if !status.success() {
                        bail!("pepsi failed with {status}");
                    }
                }
            }
        }

        Ok(())
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
        let arguments = self.arguments.join(OsStr::new(" "));

        let data_home_script = repository.data_home_script()?;

        let command_string = match self.environment {
            Environment::Native => {
                let mut command = OsString::from("cargo ");
                command.push(arguments);
                command
            }
            Environment::Sdk { version } => {
                let environment_file = format!(
                    "$({data_home_script})/sdk/{version}/environment-setup-corei7-64-aldebaran-linux",
                );
                let mut command = OsString::from(format!(". {environment_file} && cargo "));
                command.push(arguments);
                command
            }
            Environment::Docker { image } => {
                let cargo_home = format!("$({data_home_script})/container-cargo-home/");
                // TODO: Make image generic over SDK/native by modifying entry point; source SDK not here
                let pwd = Path::new("/hulk").join(&repository.root_to_current_dir()?);
                let root = repository.current_dir_to_root()?;
                let mut command = OsString::from(format!("\
                    mkdir -p {cargo_home} && \
                    docker run \
                        --volume={root}:/hulk:z \
                        --volume={cargo_home}:/naosdk/sysroots/corei7-64-aldebaran-linux/home/cargo:z \
                        --rm \
                        --interactive \
                        --tty {image} \
                        /bin/sh -c '\
                            cd {pwd} && \
                            . /naosdk/environment-setup-corei7-64-aldebaran-linux && \
                            echo $PATH && \
                            cargo \
                    ",
                    root=root.display(),
                    pwd=pwd.display(),
                ));
                command.push(arguments);
                command.push(OsStr::new("'"));
                command
            }
        };

        let mut command = match self.host {
            Host::Local => {
                let mut command = Command::new("sh");
                command.arg("-c");
                command
            }
            Host::Remote => {
                let mut command = Command::new(repository.root.join("scripts/remote_workspace"));

                for path in compiler_artifacts {
                    command.arg("--return-file").arg(path.as_ref());
                }
                let current_dir = repository.root_to_current_dir()?;
                command.arg("--cd").arg(Path::new("./").join(current_dir));
                command
            }
        };
        command.arg(command_string);

        Ok(command)
    }
}
