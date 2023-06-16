use std::path::PathBuf;

use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;
use source_analyzer::{contexts::Contexts, node::parse_rust_file, pretty::to_string_pretty};

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum Arguments {
    DumpContexts {
        /// File path to a Rust file containing a module with context structs
        file_path: PathBuf,
    },
    DumpLatest {
        /// File name to dump (may contain wildcard characters usable by glob())
        file_name: String,
    },
}

pub async fn analyze(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(file_path).wrap_err("failed to parse rust file")?;
            let context =
                Contexts::try_from_file(&file).wrap_err("failed to get contexts from rust file")?;
            let string = to_string_pretty(&context)?;
            print!("{string}");
        }
        Arguments::DumpLatest { file_name } => {
            let glob = format!("target/**/{file_name}");
            println!("{glob}");
            let file_path = repository
                .find_latest_file(&glob)
                .wrap_err("failed find latest generated file")?;
            println!("{}", file_path.display());
            PrettyPrinter::new()
                .input_file(file_path)
                .grid(true)
                .header(true)
                .line_numbers(true)
                .paging_mode(PagingMode::QuitIfOneScreen)
                .rule(true)
                .print()
                .wrap_err("failed to print file")?;
        }
    }

    Ok(())
}
