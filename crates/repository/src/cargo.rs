use std::{path::Path, process::Command};

use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};

use crate::{data_home::get_data_home, sdk::download_and_install};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Sdk { version: String },
    Docker { image: String },
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

    pub async fn setup(&self) -> Result<()> {
        if let Environment::Sdk { version } = &self.environment {
            let data_home = get_data_home().wrap_err("failed to get data home")?;

            download_and_install(version, data_home)
                .await
                .wrap_err("failed to download and install SDK")?;
        };

        Ok(())
    }

    pub fn command(&self, repository_root: impl AsRef<Path>) -> Result<Command> {
        let repository_root = repository_root.as_ref();

        // TODO: implement remote
        let command = match &self.environment {
            Environment::Native => Command::new("cargo"),
            Environment::Sdk { version } => {
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
            Environment::Docker { image } => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let cargo_home = data_home.join("container-cargo-home/");
                // TODO: This has to cd into the current pwd first
                // FIXME: Pepsi only work in repository root
                let mut command = Command::new("bash");
                command.arg("-c");
                command.arg(
                // TODO: Make image generic over SDK/native by modifying entry point; source SDK not here
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
