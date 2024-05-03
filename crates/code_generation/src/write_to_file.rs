use std::{
    env::{var, VarError},
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to get environment variable OUT_DIR")]
    Environment(#[from] VarError),
    #[error("failed to perform io")]
    Io(#[from] io::Error),
    #[error("failed to parse token stream")]
    Parse(#[from] syn::Error),
}

pub trait WriteToFile {
    fn write_to_file(self, file_name: impl AsRef<Path>) -> Result<(), Error>;
}

impl WriteToFile for TokenStream {
    fn write_to_file(self, file_name: impl AsRef<Path>) -> Result<(), Error> {
        let syntax_tree = syn::parse2(self)?;
        let pretty = prettyplease::unparse(&syntax_tree);

        let out_dir = var("OUT_DIR")?;
        let file_path = PathBuf::from(out_dir).join(file_name);
        let mut file = File::create(file_path)?;
        write!(file, "{pretty}")?;

        Ok(())
    }
}
