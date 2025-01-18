use clap::{ArgAction, Parser};
use repository::cargo::Cargo;

use super::{common::CommonOptions, heading, CargoCommand};

/// `cargo check` options which are also a subset of `cargo clippy`
#[derive(Clone, Debug, Default, Parser)]
pub struct CheckOptions {
    /// Package to build (see `cargo help pkgid`)
    #[arg(
        short = 'p',
        long = "package",
        value_name = "SPEC",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::PACKAGE_SELECTION,
    )]
    pub packages: Vec<String>,

    /// Check all packages in the workspace
    #[arg(long, help_heading = heading::PACKAGE_SELECTION)]
    pub workspace: bool,

    /// Exclude packages from the build
    #[arg(
        long,
        value_name = "SPEC",
        action = ArgAction::Append,
        help_heading = heading::PACKAGE_SELECTION,
    )]
    pub exclude: Vec<String>,

    /// Alias for workspace (deprecated)
    #[arg(long, help_heading = heading::PACKAGE_SELECTION,)]
    pub all: bool,

    /// Check only this package's library
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub lib: bool,

    /// Check only the specified binary
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bin: Vec<String>,

    /// Check all binaries
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub bins: bool,

    /// Check only the specified example
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub example: Vec<String>,

    /// Check all examples
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub examples: bool,

    /// Check only the specified test target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub test: Vec<String>,

    /// Check all tests
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub tests: bool,

    /// Check only the specified bench target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bench: Vec<String>,

    /// Check all benches
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub benches: bool,

    /// Check all targets
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub all_targets: bool,

    /// Outputs a future incompatibility report at the end of the build (unstable)
    #[arg(long)]
    pub future_incompat_report: bool,
}

impl CheckOptions {
    pub fn apply(&self, cargo: &mut Cargo) {
        for pkg in &self.packages {
            cargo.arg("--package").arg(pkg);
        }
        if self.workspace {
            cargo.arg("--workspace");
        }
        for item in &self.exclude {
            cargo.arg("--exclude").arg(item);
        }
        if self.all {
            cargo.arg("--all");
        }
        if self.lib {
            cargo.arg("--lib");
        }
        for bin in &self.bin {
            cargo.arg("--bin").arg(bin);
        }
        if self.bins {
            cargo.arg("--bins");
        }
        for example in &self.example {
            cargo.arg("--example").arg(example);
        }
        if self.examples {
            cargo.arg("--examples");
        }
        for test in &self.test {
            cargo.arg("--test").arg(test);
        }
        if self.tests {
            cargo.arg("--tests");
        }
        for bench in &self.bench {
            cargo.arg("--bench").arg(bench);
        }
        if self.benches {
            cargo.arg("--benches");
        }
        if self.all_targets {
            cargo.arg("--all-targets");
        }
        if self.future_incompat_report {
            cargo.arg("--future-incompat-report");
        }
    }
}

/// Check a local package and all of its dependencies for errors
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
}

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "check";

    fn apply(&self, cmd: &mut Cargo) {
        self.common.apply(cmd);
        self.check.apply(cmd);

        if self.release {
            cmd.arg("--release");
        }
        if self.ignore_rust_version {
            cmd.arg("--ignore-rust-version");
        }
        if self.unit_graph {
            cmd.arg("--unit-graph");
        }
    }

    fn profile(&self) -> &str {
        self.common.profile.as_deref().unwrap_or("dev")
    }
}
