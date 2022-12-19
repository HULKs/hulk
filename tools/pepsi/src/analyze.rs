use std::path::PathBuf;

use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;
use color_eyre::{eyre::WrapErr, Result};

use repository::Repository;
use source_analyzer::{parse_rust_file, Contexts, CyclerInstances, CyclerTypes, Nodes, Structs};

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
    DumpCyclerInstances,
    DumpCyclerTypes,
    DumpLatest {
        /// File name to dump (may contain wildcard characters usable by glob())
        file_name: String,
    },
    DumpNodes,
    DumpSortedNodes,
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
            let file = parse_rust_file(&file_path).wrap_err("failed to parse rust file")?;
            let context = Contexts::try_from_file(file_path, &file)
                .wrap_err("failed to get contexts from rust file")?;
            println!("{context:#?}");
        }
        Arguments::DumpCyclerInstances => {
            let cycler_instances =
                CyclerInstances::try_from_crates_directory(repository.crates_directory())
                    .wrap_err("failed to get cycler instances")?;
            println!("{cycler_instances:#?}");
        }
        Arguments::DumpCyclerTypes => {
            let cycler_types =
                CyclerTypes::try_from_crates_directory(repository.crates_directory())
                    .wrap_err("failed to get cycler types")?;
            println!("{cycler_types:#?}");
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
        Arguments::DumpNodes => {
            let nodes = Nodes::try_from_crates_directory(repository.crates_directory())
                .wrap_err("failed to get nodes")?;
            println!("{nodes:#?}");
        }
        Arguments::DumpSortedNodes => {
            let mut nodes = Nodes::try_from_crates_directory(repository.crates_directory())
                .wrap_err("failed to get nodes")?;
            nodes.sort().wrap_err("failed to sort nodes")?;
            println!("{nodes:#?}");
        }
        Arguments::DumpStructs => {
            let structs = Structs::try_from_crates_directory(repository.crates_directory())
                .wrap_err("failed to get structs")?;
            println!("{structs:#?}");
        }
    }

    Ok(())
}
