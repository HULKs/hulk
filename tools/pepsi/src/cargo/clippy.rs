use clap::Parser;
use repository::cargo::Cargo;

use super::{check::CheckOptions, common::CommonOptions, heading, CargoCommand};

/// Checks a package to catch common mistakes and improve your Rust code
#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub common: CommonOptions,

    #[command(flatten)]
    pub check: CheckOptions,

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

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "clippy";

    fn apply(&self, cargo: &mut Cargo) {
        self.common.apply(cargo);
        self.check.apply(cargo);

        if self.release {
            cargo.arg("--release");
        }
        if self.ignore_rust_version {
            cargo.arg("--ignore-rust-version");
        }
        if self.unit_graph {
            cargo.arg("--unit-graph");
        }
        if self.no_deps {
            cargo.arg("--no-deps");
        }
        if self.fix {
            cargo.arg("--fix");
        }
        if !self.args.is_empty() {
            cargo.arg("--");
            cargo.args(&self.args);
        }
    }

    fn profile(&self) -> &str {
        self.common.profile.as_deref().unwrap_or("dev")
    }
}
