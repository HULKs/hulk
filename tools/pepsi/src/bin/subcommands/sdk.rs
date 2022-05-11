use std::path::PathBuf;

use pepsi::{commands::sdk::install, logging::apply_stdout_logging};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub enum Arguments {
    Install {
        /// Force reinstallation of existing SDK
        #[structopt(long)]
        force_reinstall: bool,
        /// Alternative SDK version (e.g. "3.3")
        #[structopt(long)]
        sdk_version: Option<String>,
        /// Alternative SDK installation directory (e.g. "/opt/nao")
        #[structopt(long)]
        installation_directory: Option<PathBuf>,
    },
}

pub fn sdk(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    match arguments {
        Arguments::Install {
            force_reinstall,
            sdk_version,
            installation_directory,
        } => {
            runtime.block_on(install(
                project_root,
                force_reinstall,
                sdk_version,
                installation_directory,
                is_verbose,
            ))?;
        }
    }
    Ok(())
}
