use std::path::Path;

use clap::Args;
use color_eyre::{eyre::WrapErr, Result};
use repository::{
    cargo::{run_shell, Cargo, Environment, Executor},
    configuration::get_sdk_version,
    data_home::get_data_home,
    sdk::download_and_install,
};

#[derive(Args, Clone)]
pub struct Arguments {
    #[arg(long, default_value = "incremental")]
    pub profile: String,
    #[arg(long, default_value = None, num_args = 1..)]
    pub features: Option<Vec<String>>,
    #[arg(default_value = "nao")]
    pub target: String,
    #[arg(long)]
    pub workspace: bool,
    /// Pass through arguments to cargo
    #[arg(last = true, value_parser)]
    pub passthrough_arguments: Vec<String>,
    /// Use the SDK (automatically set when target is `nao`)
    #[arg(long)]
    pub sdk: bool,
    /// Use docker for execution, only relevant when using the SDK
    #[arg(long)]
    pub docker: bool,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}

pub async fn cargo(
    command: &str,
    arguments: Arguments,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    if arguments.remote {
        let remote_script = "./scripts/remoteWorkspace";
        let mut remote_command = remote_script.to_string();
        if command == "build" {
            let profile_name = match arguments.profile.as_str() {
                "dev" => "debug",
                other => other,
            };
            let toolchain_name = match arguments.target.as_str() {
                "nao" => "x86_64-aldebaran-linux-gnu/",
                _ => "",
            };
            remote_command.push_str(&format!(
                " --return-file target/{toolchain_name}{profile_name}/hulk_{target}",
                profile_name = profile_name,
                toolchain_name = toolchain_name,
                target = arguments.target
            ));
        }
        remote_command.push_str(&format!(
            " ./pepsi {command} --profile {profile}",
            profile = arguments.profile,
        ));
        if let Some(features) = &arguments.features {
            remote_command.push_str(&format!(
                " --features {features}",
                features = features.join(",")
            ));
        }
        if arguments.workspace {
            remote_command.push_str(" --workspace");
        }
        if arguments.sdk {
            remote_command.push_str(" --sdk");
        }
        if arguments.docker {
            remote_command.push_str(" --docker");
        }
        remote_command.push_str(&format!(" {target}", target = arguments.target));
        if !arguments.passthrough_arguments.is_empty() {
            remote_command.push_str(" -- ");
            remote_command.push_str(&arguments.passthrough_arguments.join(" "));
        }
        run_shell(&remote_command)
            .await
            .wrap_err("failed to run remote script")?;
    } else {
        let sdk_version = get_sdk_version(&repository_root)
            .await
            .wrap_err("failed to get HULK OS version")?;
        let data_home = get_data_home().wrap_err("failed to get data home")?;
        let use_sdk = arguments.sdk || (command == "build" && arguments.target == "nao");
        let cargo = if use_sdk {
            if arguments.docker || !cfg!(target_os = "linux") {
                Cargo::sdk(Environment {
                    executor: Executor::Docker,
                    version: sdk_version,
                })
            } else {
                download_and_install(&sdk_version, data_home)
                    .await
                    .wrap_err("failed to install SDK")?;
                Cargo::sdk(Environment {
                    executor: Executor::Native,
                    version: sdk_version,
                })
            }
        } else {
            Cargo::native()
        };

        let mut cargo_command = cargo.command(command, repository_root.as_ref())?;

        cargo_command.profile(&arguments.profile);

        if let Some(features) = &arguments.features {
            cargo_command.features(features);
        }

        let manifest_path = format!(
            "./crates/hulk_{target}/Cargo.toml",
            target = arguments.target
        );
        cargo_command.manifest_path(Path::new(&manifest_path))?;

        if !arguments.passthrough_arguments.is_empty() {
            cargo_command.passthrough_arguments(&arguments.passthrough_arguments);
        }

        if arguments.workspace {
            cargo_command.workspace();
            cargo_command.all_targets();
            cargo_command.all_features();
        }

        let shell_command = cargo_command.shell_command()?;
        run_shell(&shell_command)
            .await
            .wrap_err("failed to run cargo build")?;
    }
    Ok(())
}
