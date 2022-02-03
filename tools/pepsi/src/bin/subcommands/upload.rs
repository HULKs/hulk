use std::{
    fs::read_link,
    path::{Path, PathBuf},
    sync::Arc,
};

use pepsi::{
    commands::{
        self,
        compile::{compile, BuildType},
        upload::create_upload_directory,
    },
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub struct Arguments {
    /// the build type to upload, will use last compiled option if not provided
    #[structopt(long, short)]
    build_type: Option<BuildType>,
    /// do not restart hulk service after uploading
    #[structopt(long)]
    no_restart: bool,
    /// do not upload etc configuration to the nao
    #[structopt(long)]
    no_config: bool,
    /// do not remove remote files before upload
    #[structopt(long)]
    no_clean: bool,
    /// the naos to execute that command on
    #[structopt(required = true)]
    naos: Vec<NaoAddress>,
}

fn get_current_build_type(project_root: &Path) -> anyhow::Result<BuildType> {
    let path = read_link(project_root.join("build/NAO/current-buildtype"))?;
    path.file_name().unwrap().to_str().unwrap().parse()
}

pub fn upload(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    let build_type = arguments
        .build_type
        .unwrap_or(get_current_build_type(&project_root)?);
    let exit_status = runtime.block_on(compile(project_root.clone(), build_type))?;
    if !exit_status.success() {
        anyhow::bail!("Compilation failed with exit status: {}", exit_status);
    }
    let upload_dir = Arc::new(runtime.block_on(create_upload_directory(
        build_type,
        arguments.no_config,
        project_root.clone(),
    ))?);
    let no_restart = arguments.no_restart;
    let no_clean = arguments.no_clean;
    let tasks = spawn_task_per_element(&runtime, arguments.naos, |nao| {
        commands::upload::upload(
            nao.ip,
            !no_restart,
            !no_clean,
            upload_dir.clone(),
            project_root.clone(),
        )
    });
    block_on_tasks(&runtime, tasks)?;
    Ok(())
}
