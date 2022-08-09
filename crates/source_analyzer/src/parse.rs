use std::{io::Read, path::Path};

use anyhow::{anyhow, Context};
use syn::{parse_file, File};

use crate::into_anyhow_result::SynContext;

pub fn parse_rust_file<P>(file_path: P) -> anyhow::Result<File>
where
    P: AsRef<Path>,
{
    use std::fs::File;

    let mut file = File::open(&file_path)
        .with_context(|| anyhow!("Failed to open file {:?}", file_path.as_ref()))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .context("Failed to read file to string")?;
    parse_file(&buffer)
        .syn_context(&file_path)
        .context("Failed to parse file")
}
