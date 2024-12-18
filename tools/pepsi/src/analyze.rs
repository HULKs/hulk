use std::path::{Path, PathBuf};

use bat::{PagingMode, PrettyPrinter};
use clap::Subcommand;
use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};

use source_analyzer::{contexts::Contexts, node::parse_rust_file, pretty::to_string_pretty};

fn find_latest_file(path_pattern: impl AsRef<Path>) -> Result<PathBuf> {
    let matching_paths: Vec<_> = glob::glob(
        path_pattern
            .as_ref()
            .to_str()
            .ok_or_else(|| eyre!("failed to interpret path as Unicode"))?,
    )
    .wrap_err("failed to execute glob() over target directory")?
    .map(|entry| {
        let path = entry.wrap_err("failed to get glob() entry")?;
        let metadata = path
            .metadata()
            .wrap_err_with(|| format!("failed to get metadata of path {path:?}"))?;
        let modified_time = metadata.modified().wrap_err_with(|| {
            format!("failed to get modified time from metadata of path {path:?}")
        })?;
        Ok((path, modified_time))
    })
    .collect::<Result<_>>()
    .wrap_err("failed to get matching paths")?;
    let (path_with_maximal_modified_time, _modified_time) = matching_paths
        .iter()
        .max_by_key(|(_path, modified_time)| modified_time)
        .ok_or_else(|| eyre!("failed to find any matching path"))?;
    Ok(path_with_maximal_modified_time.to_path_buf())
}

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

pub async fn analyze(
    arguments: Arguments,
    repository_root: Result<impl AsRef<Path>>,
) -> Result<()> {
    match arguments {
        Arguments::DumpContexts { file_path } => {
            let file = parse_rust_file(file_path).wrap_err("failed to parse rust file")?;
            let context =
                Contexts::try_from_file(&file).wrap_err("failed to get contexts from rust file")?;
            let string = to_string_pretty(&context)?;
            print!("{string}");
        }
        Arguments::DumpLatest { file_name } => {
            let repository_root = repository_root?;
            let glob = format!("target/**/{file_name}");
            println!("{glob}");
            let file_path = find_latest_file(repository_root.as_ref().join(glob))
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
