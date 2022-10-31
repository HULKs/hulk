mod checks;
mod output;
mod syn_context;

use std::{io::Read, path::Path};

use checks::{empty_lines, mod_use_order};
use color_eyre::{eyre::{bail, Context}, Result};
use syn::{parse_file, File};

use crate::syn_context::SynContext;

fn main() -> Result<()> {
    let success =
        check("src/control/modules/localization.rs").wrap_err("failed to check Rust file")?;
    if !success {
        bail!("at least one check failed");
    }
    Ok(())
}

fn check<P>(file_path: P) -> Result<bool>
where
    P: AsRef<Path>,
{
    let (buffer, file) = parse_rust_file(&file_path).wrap_err("failed to parse Rust file")?;
    let success = mod_use_order::check(&file_path, &buffer, &file);
    let success = empty_lines::check(&file_path, &buffer, &file) && success;
    Ok(success)
}

fn parse_rust_file<P>(file_path: P) -> Result<(String, File)>
where
    P: AsRef<Path>,
{
    use std::fs::File;

    let mut file =
        File::open(&file_path).wrap_err(format!("failed to open file {:?}", file_path.as_ref()))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .wrap_err("failed to read file to string")?;
    Ok((
        buffer.clone(),
        parse_file(&buffer)
            .syn_context(&file_path)
            .wrap_err("failed to parse file")?,
    ))
}
