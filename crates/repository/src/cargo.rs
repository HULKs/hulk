use core::fmt;
use std::{
    ffi::{OsStr, OsString},
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::{eyre::Context, Result};
use tokio::process::Command;

use crate::{
    sdk::{build_sdk_container, image_exists_locally, SDKImage},
    Repository,
};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Podman { sdk_image: SDKImage },
    Docker { sdk_image: SDKImage },
}

impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Environment::Native => write!(f, "Native"),
            Environment::Podman { sdk_image } => {
                write!(f, "Podman ({})", sdk_image.name_tagged())
            }
            Environment::Docker { sdk_image } => {
                write!(f, "Docker ({})", sdk_image.name_tagged())
            }
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
        if let Environment::Podman { sdk_image } = &self.environment {
            match self.host {
                Host::Local => {
                    if !image_exists_locally(sdk_image).await {
                        build_sdk_container(repository, sdk_image)
                            .await
                            .wrap_err("failed to build SDK container")?
                    }
                }
                Host::Remote => {
                    // let mut command =
                    //     Command::new(repository.root.join("scripts/remote_workspace"));

                    // let status = command
                    //     .arg("pepsi")
                    //     .arg("sdk")
                    //     .adownload_and_installrg("install")
                    //     .arg("--version")
                    //     .arg(version)
                    //     .status()
                    //     .await
                    //     .wrap_err("failed to run pepsi")?;

                    // if !status.success() {
                    //     bail!("pepsi failed with {status}");
                    // }
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
            Environment::Podman { sdk_image } => {
                let cargo_home = format!("$({data_home_script})/container-cargo-home/");
                // TODO: Make image generic over SDK/native by modifying entry point; source SDK not here
                let pwd = Path::new("/hulk").join(&repository.root_to_current_dir()?);
                let root = repository.current_dir_to_root()?;
                let tagged_image_name = sdk_image.name_tagged();
                let mut command = OsString::from(format!(
                    "\
                    mkdir -p {cargo_home}/git && \
                    mkdir -p {cargo_home}/registry && \
                    podman run \
                        --volume={root}:/hulk:z \
                        --volume={cargo_home}/git:/root/.cargo/git:z \
                        --volume={cargo_home}/registry:/root/.cargo/registry:z \
                        --net=host \
                        --rm \
                        --interactive \
                        --pull=never \
                        --tty \
                        {tagged_image_name} \
                        /bin/sh -c '\
                            cd {pwd} && \
                            echo $PATH && \
                            /root/.cargo/bin/cargo \
                    ",
                    root = root.display(),
                    pwd = pwd.display(),
                ));
                command.push(arguments);
                command.push(OsStr::new("'"));
                command
            }
            Environment::Docker { sdk_image } => {
                let cargo_home = format!("$({data_home_script})/container-cargo-home/");
                // TODO: Make image generic over SDK/native by modifying entry point; source SDK not here
                let pwd = Path::new("/hulk").join(&repository.root_to_current_dir()?);
                let root = repository.current_dir_to_root()?;
                let tagged_image_name = sdk_image.name_tagged();
                let mut command = OsString::from(format!(
                    "\
                    mkdir -p {cargo_home}/git && \
                    mkdir -p {cargo_home}/registry && \
                    docker run \
                        --volume={root}:/hulk:z \
                        --volume={cargo_home}/git:/root/.cargo/git:z \
                        --volume={cargo_home}/registry:/root/.cargo/registry:z \
                        --rm \
                        --interactive \
                        --pull=never \
                        --tty \
                        {tagged_image_name} \
                        /bin/sh -c '\
                            cd {pwd} && \
                            echo $PATH && \
                            cargo \
                    ",
                    root = root.display(),
                    pwd = pwd.display(),
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
