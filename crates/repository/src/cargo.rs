use core::fmt;
use std::{
    env::current_dir,
    ffi::{OsStr, OsString},
    fmt::{Display, Formatter},
    path::Path,
};

use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};
use pathdiff::diff_paths;
use tokio::process::Command;

use crate::{data_home::get_data_home, sdk::download_and_install};

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

    pub async fn setup(&self, repository_root: impl AsRef<Path>) -> Result<()> {
        if let Environment::Sdk { version } = &self.environment {
            match self.host {
                Host::Local => {
                    let data_home = get_data_home().wrap_err("failed to get data home")?;

                    download_and_install(version, data_home)
                        .await
                        .wrap_err("failed to download and install SDK")?;
                }
                Host::Remote => {
                    let mut command =
                        Command::new(repository_root.as_ref().join("scripts/remoteWorkspace"));

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
        repository_root: impl AsRef<Path>,
        compiler_artifacts: &[impl AsRef<Path>],
    ) -> Result<Command> {
        let repository_root = repository_root.as_ref();

        let arguments = self.arguments.join(OsStr::new(" "));

        let relative_pwd = diff_paths(
            current_dir().wrap_err("failed to get current directory")?,
            repository_root,
        )
        .wrap_err("failed to express current directory relative to repository root")?;

        let command_string = match self.environment {
            Environment::Native => {
                let mut command = OsString::from("cargo ");
                command.push(arguments);
                command
            }
            Environment::Sdk { version } => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let environment_file = &data_home.join(format!(
                    "sdk/{version}/environment-setup-corei7-64-aldebaran-linux"
                ));
                let sdk_environment_setup = environment_file
                    .to_str()
                    .wrap_err("failed to convert sdk environment setup path to string")?;
                let mut command = OsString::from(format!(". {sdk_environment_setup} && cargo "));
                command.push(arguments);
                command
            }
            Environment::Docker { image } => {
                let data_home = get_data_home().wrap_err("failed to get data home")?;
                let cargo_home = data_home.join("container-cargo-home/");
                // FIXME: Pepsi only work in repository root
                // TODO: Make image generic over SDK/native by modifying entry point; source SDK not here
                let pwd = Path::new("/hulk").join(&relative_pwd);
                let mut command = OsString::from(format!("\
                    mkdir -p {cargo_home} && \
                    docker run \
                        --volume={repository_root}:/hulk:z \
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
                    repository_root=repository_root.display(),
                    cargo_home=cargo_home.display(),
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
                let mut command = Command::new(repository_root.join("scripts/remoteWorkspace"));

                for path in compiler_artifacts {
                    command.arg("--return-file").arg(path.as_ref());
                }
                command.arg("--cd").arg(Path::new("./").join(relative_pwd));
                command
            }
        };
        command.arg(command_string);

        Ok(command)
    }
}
