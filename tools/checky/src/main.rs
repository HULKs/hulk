use std::path::PathBuf;

use checks::check;
use clap::Parser;
use color_eyre::{eyre::Context, Result};
use rust_file::RustFile;
use walkdir::WalkDir;

mod checks;
mod rust_file;
mod syn_context;

#[derive(Parser, Debug)]
struct Arguments {
    paths: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    for path in arguments.paths {
        for entry in WalkDir::new(&path) {
            let entry =
                entry.wrap_err_with(|| format!("failed to walk directory {}", path.display()))?;

            if !entry.file_type().is_file() {
                continue;
            }

            if !entry
                .path()
                .extension()
                .map(|extension| extension == "rs")
                .unwrap_or_default()
            {
                continue;
            }

            let file = RustFile::try_parse(entry.path())
                .wrap_err_with(|| format!("failed to parse {}", entry.path().display()))?;

            for report in check(&file) {
                report
                    .eprint((file.source_id.as_str(), file.source.clone()))
                    .wrap_err("failed to output report")?;
            }
        }
    }

    Ok(())
}
