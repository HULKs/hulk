use std::path::PathBuf;

use anyhow::bail;
use pepsi::{
    commands::{
        build::{build as build_command, BuildType, Target},
        sdk::install,
    },
    logging::apply_stdout_logging,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub struct Arguments {
    #[structopt(long, default_value, possible_values = &BuildType::variants())]
    build_type: BuildType,
    #[structopt(long, default_value, possible_values = &Target::variants())]
    target: Target,
    #[structopt(long)]
    no_sdk_installation: bool,
}

pub fn build(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;

    if !arguments.no_sdk_installation && arguments.target == Target::NAO {
        runtime.block_on(install(project_root.clone(), false, None, None, is_verbose))?;
    }

    let exit_status = runtime.block_on(build_command(
        project_root,
        arguments.build_type,
        arguments.target,
        is_verbose,
    ))?;
    if !exit_status.success() {
        bail!("Build failed with exit status: {}", exit_status);
    }

    Ok(())
}
