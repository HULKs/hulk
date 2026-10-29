use core::fmt;
use std::{
    ffi::{OsStr, OsString},
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::{
    Result,
    eyre::{Context, bail},
};
use tokio::process::Command;

use crate::{
    Repository,
    sdk::{ContainerRuntime, SDKImage, build_sdk_container_with, image_exists_locally_with},
};

#[derive(Debug, Clone)]
pub enum Environment {
    Native,
    Podman { sdk_image: SDKImage },
    Docker { sdk_image: SDKImage },
}

pub const CONTAINER_TARGET_DIRECTORY: &str = "target/container";

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

impl Environment {
    fn container(&self) -> Option<(ContainerRuntime, &SDKImage)> {
        match self {
            Environment::Native => None,
            Environment::Podman { sdk_image } => Some((ContainerRuntime::Podman, sdk_image)),
            Environment::Docker { sdk_image } => Some((ContainerRuntime::Docker, sdk_image)),
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

    pub async fn setup(&self, repository: &Repository) -> Result<()> {
        match (&self.environment, &self.host) {
            (environment, Host::Local) => {
                if let Some((runtime, sdk_image)) = environment.container()
                    && !image_exists_locally_with(sdk_image, runtime).await?
                {
                    build_sdk_container_with(repository, sdk_image, runtime)
                        .await
                        .wrap_err("failed to build SDK container")?
                }
            }
            (Environment::Podman { sdk_image }, Host::Remote) => {
                let mut command = Command::new(repository.root.join("scripts/remote_workspace"));

                let status = command
                    .arg(format!("podman image exists {}", sdk_image.name_tagged()))
                    .arg("|| ./pepsi sdk build --image")
                    .arg(sdk_image.name_tagged())
                    .status()
                    .await
                    .wrap_err("failed to run pepsi")?;

                if !status.success() {
                    bail!("pepsi failed with {status}");
                }
            }
            (Environment::Docker { .. }, Host::Remote) | (Environment::Native, _) => {}
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
            Environment::Podman { sdk_image } => build_container_command_string(
                repository,
                data_home_script,
                ContainerRuntime::Podman,
                sdk_image,
                arguments,
            )?,
            Environment::Docker { sdk_image } => build_container_command_string(
                repository,
                data_home_script,
                ContainerRuntime::Docker,
                sdk_image,
                arguments,
            )?,
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

fn build_container_command_string(
    repository: &Repository,
    data_home_script: String,
    runtime: ContainerRuntime,
    sdk_image: SDKImage,
    arguments: OsString,
) -> Result<OsString> {
    let cargo_home = format!("$({data_home_script})/container-cargo-home/");
    let pwd = Path::new("/hulk").join(&repository.root_to_current_dir()?);
    let root = repository.current_dir_to_root()?;
    let tagged_image_name = sdk_image.name_tagged();
    let mut command = OsString::from(build_command_string(
        runtime.binary(),
        cargo_home,
        root.display().to_string(),
        tagged_image_name,
        pwd.display().to_string(),
        runtime.allocate_tty(),
    ));
    command.push(arguments);
    command.push(OsStr::new("\""));

    Ok(command)
}

fn build_command_string(
    container_runtime: &str,
    cargo_home: String,
    root: String,
    tagged_image_name: String,
    pwd: String,
    allocate_tty: bool,
) -> String {
    let tty_argument = if allocate_tty {
        "                --tty \\\n"
    } else {
        ""
    };

    format!(
        "\
            mkdir -p {cargo_home}/git && \
            mkdir -p {cargo_home}/registry && \
            {container_runtime} run \
                --env=CARGO_TARGET_DIR=/hulk/{CONTAINER_TARGET_DIRECTORY} \
                --volume={root}:/hulk:z \
                --volume={cargo_home}/git:/root/.cargo/git:z \
                --volume={cargo_home}/registry:/root/.cargo/registry:z \
                --rm \
                --network=host \
                --interactive \
                --pull=never \
{tty_argument}\
                {tagged_image_name} \
                /bin/sh -c \"\
                    cd {pwd} && \
                    cargo \
        "
    )
}
