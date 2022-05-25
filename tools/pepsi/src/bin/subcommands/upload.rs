use std::{path::PathBuf, sync::Arc};

use anyhow::bail;
use pepsi::{
    commands::{
        self,
        build::{build, BuildType, Target},
        sdk::install,
        upload::create_upload_directory,
    },
    logging::apply_stdout_logging,
    naossh::fix_ssh_key_permissions,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub struct Arguments {
    /// the build type to upload
    #[structopt(long, default_value, possible_values = &BuildType::variants())]
    build_type: BuildType,
    /// do not try to install the latest SDK when building
    #[structopt(long)]
    no_sdk_installation: bool,
    /// do not build before uploading
    #[structopt(long)]
    no_build: bool,
    /// do not restart hulk service after uploading
    #[structopt(long)]
    no_restart: bool,
    /// do not upload etc configuration to the NAO
    #[structopt(long)]
    no_configuration: bool,
    /// do not remove remote files before upload
    #[structopt(long)]
    no_clean: bool,
    /// the NAOs to execute that command on
    #[structopt(required = true)]
    naos: Vec<NaoAddress>,
}

pub fn upload(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;

    if !arguments.no_sdk_installation {
        runtime.block_on(install(project_root.clone(), false, None, None, is_verbose))?;
    }

    if !arguments.no_build {
        let exit_status = runtime.block_on(build(
            project_root.clone(),
            arguments.build_type,
            Target::NAO,
            is_verbose,
        ))?;
        if !exit_status.success() {
            bail!("Build failed with exit status: {}", exit_status);
        }
    }

    let upload_directory = Arc::new(runtime.block_on(create_upload_directory(
        arguments.build_type,
        arguments.no_configuration,
        project_root.clone(),
    ))?);

    fix_ssh_key_permissions(project_root.clone())?;

    let tasks = spawn_task_per_element(&runtime, arguments.naos, |nao| {
        commands::upload::upload(
            nao.ip,
            !arguments.no_restart,
            !arguments.no_clean,
            upload_directory.clone(),
            project_root.clone(),
        )
    });
    block_on_tasks(&runtime, tasks)?;

    Ok(())
}
