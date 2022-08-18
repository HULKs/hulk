use anyhow::Context;
use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;

use repository::Repository;

#[derive(Subcommand)]
pub enum Arguments {
    DumpLatest {
        /// File name to dump (may contain wildcard characters usable by glob())
        file_name: String,
    },
}

pub async fn analyze(arguments: Arguments, repository: &Repository) -> anyhow::Result<()> {
    match arguments {
        Arguments::DumpLatest { file_name } => {
            let file_path = repository
                .find_latest_generated_file(&file_name)
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
    }

    Ok(())
}
