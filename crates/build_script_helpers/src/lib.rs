use std::{env::var, fs::File, io::Write, path::PathBuf};

use color_eyre::{eyre::WrapErr, Result};
use proc_macro2::TokenStream;

pub fn write_token_stream(file_name: &str, token_stream: TokenStream) -> Result<()> {
    let file_path =
        PathBuf::from(var("OUT_DIR").wrap_err("failed to get environment variable OUT_DIR")?)
            .join(file_name);

    {
        let mut file = File::create(&file_path)
            .wrap_err_with(|| format!("failed create file {file_path:?}"))?;
        write!(file, "{token_stream}")
            .wrap_err_with(|| format!("failed to write to file {file_path:?}"))?;
    }

    Ok(())
}
