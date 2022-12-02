use std::{env::var, fs::File, io::Write, path::PathBuf, process::Command};

use color_eyre::{
    eyre::{bail, WrapErr},
    Result,
};
use proc_macro2::TokenStream;

pub fn write_token_stream(file_name: &str, token_stream: TokenStream) -> Result<()> {
    let file_path =
        PathBuf::from(var("OUT_DIR").wrap_err("failed to get environment variable OUT_DIR")?)
            .join(file_name);

    {
        let mut file = File::create(&file_path)
            .wrap_err_with(|| format!("failed create file {file_path:?}"))?;
        write!(file, "{}", token_stream)
            .wrap_err_with(|| format!("failed to write to file {file_path:?}"))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .wrap_err("failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}
