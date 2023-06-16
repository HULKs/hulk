use std::path::PathBuf;

use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use source_analyzer::{contexts::Contexts, node::parse_rust_file, pretty::to_string_pretty};

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum Arguments {
    DumpContexts {
        /// File path to a Rust file containing a module with context structs
        file_path: PathBuf,
    },
}

pub async fn analyze(arguments: Arguments) -> Result<()> {
    match arguments {
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(file_path).wrap_err("failed to parse rust file")?;
            let context =
                Contexts::try_from_file(&file).wrap_err("failed to get contexts from rust file")?;
            let string = to_string_pretty(&context)?;
            print!("{string}");
        }
    }

    Ok(())
}
