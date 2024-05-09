use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use ariadne::Source;
use color_eyre::{eyre::WrapErr, Result};
use syn::parse_file;

use crate::syn_context::SynContext;

#[derive(Clone, Debug)]
pub struct RustFile {
    pub path: PathBuf,
    pub source_id: String,
    pub source: Source,
    pub file: syn::File,
}

impl RustFile {
    pub fn try_parse(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = fs::File::open(&path)
            .wrap_err_with(|| format!("failed to open file {}", path.as_ref().display()))?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).wrap_err_with(|| {
            format!("failed to read file {} to string", path.as_ref().display())
        })?;
        let path = path.as_ref().to_path_buf();
        let source_id = path.display().to_string();
        let source = Source::from(buffer);
        let file = parse_file(source.text())
            .syn_context(&path)
            .wrap_err_with(|| format!("failed to parse file {}", path.display()))?;
        Ok(Self {
            path,
            source_id,
            source,
            file,
        })
    }
}
