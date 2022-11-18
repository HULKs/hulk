use std::path::PathBuf;

use anyhow::Context;
use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;

use repository::Repository;
use source_analyzer::{parse_rust_file, Contexts, CyclerInstances, CyclerTypes, Modules, Structs};

#[derive(Subcommand)]
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
    DumpModules,
    DumpStructs,
}

pub async fn analyze(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    match arguments {
        Arguments::DumpBuildScriptOutput {
            crate_name,
            file_name,
        } => {
            let prefix = format!("target/**/{crate_name}-*/**");
            let file_path = repository
                .find_latest_file(&prefix, &file_name)
                .context("Failed find latest build script output")?;
            PrettyPrinter::new()
                .input_file(file_path)
                .grid(true)
                .header(true)
                .line_numbers(true)
                .paging_mode(PagingMode::QuitIfOneScreen)
                .rule(true)
                .print()
                .context("Failed to print file")?;
        }
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(&file_path).context("Failed to parse rust file")?;
            let context = Contexts::try_from_file(file_path, &file)
                .context("Failed to get contexts from rust file")?;
            println!("{context:#?}");
        }
        Arguments::DumpCyclerInstances => {
            let cycler_instances =
                CyclerInstances::try_from_crates_directory(repository.get_crates_directory())
                    .context("Failed to get cycler instances")?;
            println!("{cycler_instances:#?}");
        }
        Arguments::DumpCyclerTypes => {
            let cycler_types =
                CyclerTypes::try_from_crates_directory(repository.get_crates_directory())
                    .context("Failed to get cycler types")?;
            println!("{cycler_types:#?}");
        }
        Arguments::DumpLatest { file_name } => {
            let file_path = repository
                .find_latest_file("target/**/out/**", &file_name)
                .context("Failed find latest generated file")?;
            PrettyPrinter::new()
                .input_file(file_path)
                .grid(true)
                .header(true)
                .line_numbers(true)
                .paging_mode(PagingMode::QuitIfOneScreen)
                .rule(true)
                .print()
                .context("Failed to print file")?;
        }
        Arguments::DumpModules => {
            let modules = Modules::try_from_crates_directory(repository.get_crates_directory())
                .context("Failed to get modules")?;
            println!("{modules:#?}");
        }
        Arguments::DumpStructs => {
            let structs = Structs::try_from_crates_directory(repository.get_crates_directory())
                .context("Failed to get structs")?;
            println!("{structs:#?}");
        }
    }

    Ok(())
}
