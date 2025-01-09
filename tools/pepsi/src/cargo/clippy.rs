use std::{
    path::{Path, PathBuf},
    process::Command,
};

use clap::Parser;
use color_eyre::eyre::{bail, Context, Result};

use crate::CargoArguments;

use super::{cargo, check::CheckOptions, common::CommonOptions, heading};

/// Checks a package to catch common mistakes and improve your Rust code
#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub common: CommonOptions,

    #[command(flatten)]
    pub check: CheckOptions,

    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH", help_heading = heading::MANIFEST_OPTIONS)]
    pub manifest_path: Option<PathBuf>,

    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'r', long, help_heading = heading::COMPILATION_OPTIONS)]
    pub release: bool,

    /// Ignore `rust-version` specification in packages
    #[arg(long)]
    pub ignore_rust_version: bool,

    /// Output build graph in JSON (unstable)
    #[arg(long, help_heading = heading::COMPILATION_OPTIONS)]
    pub unit_graph: bool,

    /// Ignore dependencies, run only on crate
    #[arg(long)]
    pub no_deps: bool,

    /// Automatically apply lint suggestions (see `cargo help clippy`)
    #[arg(long)]
    pub fix: bool,

    /// Arguments passed to rustc.
    #[arg(value_name = "args", trailing_var_arg = true, num_args = 0..)]
    pub args: Vec<String>,
}

impl Arguments {
    fn apply<'a>(&self, cmd: &'a mut Command) -> &'a mut Command {
        cmd.arg("clippy");

        self.common.apply(cmd);
        self.check.apply(cmd);

        if let Some(path) = self.manifest_path.as_ref() {
            cmd.arg("--manifest-path").arg(path);
        }
        if self.release {
            cmd.arg("--release");
        }
        if self.ignore_rust_version {
            cmd.arg("--ignore-rust-version");
        }
        if self.unit_graph {
            cmd.arg("--unit-graph");
        }
        if self.no_deps {
            cmd.arg("--no-deps");
        }
        if self.fix {
            cmd.arg("--fix");
        }
        if !self.args.is_empty() {
            cmd.arg("--");
            cmd.args(&self.args);
        }

        cmd
    }
}

pub async fn clippy(
    arguments: CargoArguments<Arguments>,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let mut cargo_command = cargo(arguments.environment, repository_root)
        .await
        .wrap_err("failed to build cargo command")?;

    // TODO: Build extension trait for readability
    arguments.cargo.apply(&mut cargo_command);

    let status = tokio::process::Command::from(cargo_command)
        .status()
        .await
        .wrap_err("failed to run cargo build")?;

    if !status.success() {
        bail!("pepsi build failed with {status}");
    }

    Ok(())
}
