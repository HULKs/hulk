use std::{path::Path, process::Command, str::FromStr};

use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};

use crate::data_home::get_data_home;

#[derive(Debug, Clone)]
pub enum Sdk {
    Installed { version: String },
    Docker { image: String },
}

pub enum Environment {
    Native,
    Sdk(Sdk),
}

pub enum Host {
    Local,
    Remote,
}

pub struct Cargo {
    host: Host,
    environment: Environment,
}

impl Cargo {
    pub fn local(environment: Environment) -> Self {
        Self {
            host: Host::Local,
            environment,
        }
    }

    pub fn remote(environment: Environment) -> Self {
        Self {
            host: Host::Remote,
            environment,
        }
    }

    pub fn command(self, repository_root: impl AsRef<Path>) -> Result<Command> {
        let repository_root = repository_root.as_ref();

        // TODO: implement remote
        let command = match self.environment {
            Environment::Native => Command::new("cargo"),
            Environment::Sdk(Sdk::Installed { version }) => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let environment_file = &data_home.join(format!(
                    "sdk/{version}/environment-setup-corei7-64-aldebaran-linux"
                ));
                let sdk_environment_setup = environment_file
                    .to_str()
                    .wrap_err("failed to convert sdk environment setup path to string")?;
                let mut command = Command::new("bash");
                command
                    .arg("-c")
                    .arg(format!(". {sdk_environment_setup} && cargo $@"))
                    .arg("cargo");
                command
            }
            Environment::Sdk(Sdk::Docker { image }) => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let cargo_home = data_home.join("container-cargo-home/");
                // TODO: This has to cd into the current pwd first
                let mut command = Command::new("bash");
                command.arg("-c");
                command.arg(
                format!("\
                    mkdir -p {cargo_home} && \
                    docker run \
                        --volume={repository_root}:/hulk:z \
                        --volume={cargo_home}:/naosdk/sysroots/corei7-64-aldebaran-linux/home/cargo:z \
                        --rm \
                        --interactive \
                        --tty {image} \
                        /bin/bash -c \"\
                            cd /hulk && \
                            . /naosdk/environment-setup-corei7-64-aldebaran-linux && \
                            cargo $@\
                        \"
                    ",
                    repository_root=repository_root.to_str().wrap_err("failed to convert repository root to string")?,
                    cargo_home=cargo_home.to_str().wrap_err("failed to convert cargo home to string")?,
                )).arg("cargo");
                command
            }
        };
        Ok(command)
    }
}
