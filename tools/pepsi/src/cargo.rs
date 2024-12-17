use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Args;
use color_eyre::{eyre::Context, Result};
use environment::EnvironmentArguments;
use repository::{
    cargo::{Cargo, Environment},
    configuration::read_sdk_version,
};

pub mod build;
pub mod check;
pub mod clippy;
pub mod common;
pub mod environment;
pub mod run;
mod heading {
    pub const PACKAGE_SELECTION: &str = "Package Selection";
    pub const TARGET_SELECTION: &str = "Target Selection";
    pub const FEATURE_SELECTION: &str = "Feature Selection";
    pub const COMPILATION_OPTIONS: &str = "Compilation Options";
    pub const MANIFEST_OPTIONS: &str = "Manifest Options";
}

pub trait CargoCommand {
    fn apply<'a>(&self, cmd: &'a mut Command) -> &'a mut Command;
}

pub async fn cargo<CargoArguments>(
    environment_arguments: EnvironmentArguments,
    cargo_arguments: CargoArguments,
    repository_root: impl AsRef<Path>,
) -> Result<()>
where
    CargoArguments: clap::Args + CargoCommand,
{
    let repository_root = repository_root.as_ref();

    let sdk_version = read_sdk_version(repository_root)
        .await
        .wrap_err("failed to read SDK version")?;

    let environment = match environment_arguments.sdk {
        Some(executor) => Environment::Sdk {
            executor,
            version: sdk_version,
        },
        None => Environment::Native,
    };
    let cargo = if environment_arguments.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };

    let mut command = cargo
        .command(repository_root)
        .wrap_err("failed to create cargo command")?;
    cargo_arguments.apply(&mut command);

    tokio::process::Command::from(command)
        .status()
        .await
        .wrap_err("failed to run cargo")?;
    Ok(())
}

//pub async fn cargo(
//    command: &str,
//    arguments: Arguments,
//    repository_root: impl AsRef<Path>,
//) -> Result<()> {
//    if arguments.remote {
//        let remote_script = "./scripts/remoteWorkspace";
//        let mut remote_command = remote_script.to_string();
//        if command == "build" {
//            let profile_name = match arguments.profile.as_str() {
//                "dev" => "debug",
//                other => other,
//            };
//            let toolchain_name = match arguments.target.as_str() {
//                "nao" => "x86_64-aldebaran-linux-gnu/",
//                _ => "",
//            };
//            remote_command.push_str(&format!(
//                " --return-file target/{toolchain_name}{profile_name}/hulk_{target}",
//                profile_name = profile_name,
//                toolchain_name = toolchain_name,
//                target = arguments.target
//            ));
//        }
//        remote_command.push_str(&format!(
//            " ./pepsi {command} --profile {profile}",
//            profile = arguments.profile,
//        ));
//        if let Some(features) = &arguments.features {
//            remote_command.push_str(&format!(
//                " --features {features}",
//                features = features.join(",")
//            ));
//        }
//        if arguments.workspace {
//            remote_command.push_str(" --workspace");
//        }
//        if arguments.sdk {
//            remote_command.push_str(" --sdk");
//        }
//        if arguments.docker {
//            remote_command.push_str(" --docker");
//        }
//        remote_command.push_str(&format!(" {target}", target = arguments.target));
//        if !arguments.passthrough_arguments.is_empty() {
//            remote_command.push_str(" -- ");
//            remote_command.push_str(&arguments.passthrough_arguments.join(" "));
//        }
//        run_shell(&remote_command)
//            .await
//            .wrap_err("failed to run remote script")?;
//    } else {
//        let sdk_version = read_sdk_version(&repository_root)
//            .await
//            .wrap_err("failed to get HULK OS version")?;
//        let data_home = get_data_home().wrap_err("failed to get data home")?;
//        let use_sdk = arguments.sdk || (command == "build" && arguments.target == "nao");
//        let cargo = if use_sdk {
//            if arguments.docker || !cfg!(target_os = "linux") {
//                Cargo::sdk(Environment {
//                    executor: Executor::Docker,
//                    version: sdk_version,
//                })
//            } else {
//                download_and_install(&sdk_version, data_home)
//                    .await
//                    .wrap_err("failed to install SDK")?;
//                Cargo::sdk(Environment {
//                    executor: Executor::Native,
//                    version: sdk_version,
//                })
//            }
//        } else {
//            Cargo::native()
//        };
//
//        let mut cargo_command = cargo.command(command, repository_root.as_ref())?;
//
//        cargo_command.profile(&arguments.profile);
//
//        if let Some(features) = &arguments.features {
//            cargo_command.features(features);
//        }
//
//        let manifest_path = format!(
//            "./crates/hulk_{target}/Cargo.toml",
//            target = arguments.target
//        );
//        cargo_command.manifest_path(Path::new(&manifest_path))?;
//
//        if !arguments.passthrough_arguments.is_empty() {
//            cargo_command.passthrough_arguments(&arguments.passthrough_arguments);
//        }
//
//        if arguments.workspace {
//            cargo_command.workspace();
//            cargo_command.all_targets();
//            cargo_command.all_features();
//        }
//
//        let shell_command = cargo_command.shell_command()?;
//        run_shell(&shell_command)
//            .await
//            .wrap_err("failed to run cargo build")?;
//    }
//    Ok(())
//}
