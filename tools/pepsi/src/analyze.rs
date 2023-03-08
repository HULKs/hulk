use std::path::PathBuf;

use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;
use source_analyzer::{
    contexts::Contexts, cycler::Cyclers, node::parse_rust_file, structs::Structs,
};

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum Arguments {
    DumpBuildScriptOutput {
        /// Crate name whose build script output to dump (may contain wildcard characters usable by glob())
        crate_name: String,
        /// File name to dump (may contain wildcard characters usable by glob())
        file_name: String,
    },
    DumpContexts {
        /// File path to a Rust file containing a module with context structs
        file_path: PathBuf,
    },
    DumpCyclers,
    DumpLatest {
        /// File name to dump (may contain wildcard characters usable by glob())
        file_name: String,
    },
    DumpStructs,
}

pub async fn analyze(arguments: Arguments, repository: &Repository) -> Result<()> {
    match arguments {
        Arguments::DumpBuildScriptOutput {
            crate_name,
            file_name,
        } => {
            let file_path = repository
                .find_latest_file(&format!("target/**/{crate_name}-*/**/{file_name}"))
                .wrap_err("failed find latest build script output")?;
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
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(file_path).wrap_err("failed to parse rust file")?;
            let context =
                Contexts::try_from_file(&file).wrap_err("failed to get contexts from rust file")?;
            println!("{context}");
        }
        Arguments::DumpCyclers => {
            let cyclers = Cyclers::try_from_directory("crates/hulk/")?;
            println!("{cyclers}");
        }
        Arguments::DumpLatest { file_name } => {
            let file_path = repository
                .find_latest_file(&format!("target/**/out/**/{file_name}"))
                .wrap_err("failed find latest generated file")?;
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
        Arguments::DumpStructs => {
            let cyclers = Cyclers::try_from_directory("crates/hulk/")?;
            let structs = Structs::try_from_cyclers(&cyclers)?;
            println!("{structs:#?}");
        }
    }

    Ok(())
}
