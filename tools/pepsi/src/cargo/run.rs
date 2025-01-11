use clap::{ArgAction, Parser};
use repository::cargo::Cargo;

use super::common::CommonOptions;
use super::{heading, CargoCommand};

#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
pub struct Arguments {
    #[command(flatten)]
    pub common: CommonOptions,

    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'r', long, help_heading = heading::COMPILATION_OPTIONS)]
    pub release: bool,

    /// Ignore `rust-version` specification in packages
    #[arg(long)]
    pub ignore_rust_version: bool,

    /// Output build graph in JSON (unstable)
    #[arg(long, help_heading = heading::COMPILATION_OPTIONS)]
    pub unit_graph: bool,

    /// Package to run (see `cargo help pkgid`)
    #[arg(
        short = 'p',
        long = "package",
        value_name = "SPEC",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::PACKAGE_SELECTION,
    )]
    pub packages: Vec<String>,

    /// Run the specified binary
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bin: Vec<String>,

    /// Run the specified example
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub example: Vec<String>,

    /// Arguments for the binary to run
    #[arg(value_name = "args", trailing_var_arg = true, num_args = 0..)]
    pub args: Vec<String>,
}

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "run";

    fn apply(&self, cargo: &mut Cargo) {
        self.common.apply(cargo);

        if self.release {
            cargo.arg("--release");
        }
        if self.ignore_rust_version {
            cargo.arg("--ignore-rust-version");
        }
        if self.unit_graph {
            cargo.arg("--unit-graph");
        }
        for pkg in &self.packages {
            cargo.arg("--package").arg(pkg);
        }
        for bin in &self.bin {
            cargo.arg("--bin").arg(bin);
        }
        for example in &self.example {
            cargo.arg("--example").arg(example);
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
