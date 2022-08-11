use std::{env::var, fs::File, io::Write, path::PathBuf, process::Command};

use anyhow::{anyhow, bail, Context};
use proc_macro2::TokenStream;

pub fn write_token_stream(file_name: &str, token_stream: TokenStream) -> anyhow::Result<()> {
    let file_path =
        PathBuf::from(var("OUT_DIR").context("Failed to get environment variable OUT_DIR")?)
            .join(file_name);

    {
        let mut file = File::create(&file_path)
            .with_context(|| anyhow!("Failed create file {file_path:?}"))?;
        write!(file, "{}", token_stream)
            .with_context(|| anyhow!("Failed to write to file {file_path:?}"))?;
    }

    let status = Command::new("rustfmt")
        .arg(file_path)
        .status()
        .context("Failed to execute rustfmt")?;
    if !status.success() {
        bail!("rustfmt did not exit with success");
    }

    Ok(())
}
