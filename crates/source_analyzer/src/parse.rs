use std::{fs, io::Read, path::Path};

use color_eyre::{eyre::WrapErr, Result};
use syn::{self, parse_file};

use crate::into_eyre_result::SynContext;

pub fn parse_rust_file<P>(file_path: P) -> Result<syn::File>
where
    P: AsRef<Path>,
{
    let mut file = fs::File::open(&file_path)
        .wrap_err_with(|| format!("failed to open file {:?}", file_path.as_ref()))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .wrap_err("failed to read file to string")?;
    parse_file(&buffer)
        .syn_context(&file_path)
        .wrap_err("failed to parse file")
}
