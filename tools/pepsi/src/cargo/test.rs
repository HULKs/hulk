use clap::{ArgAction, Parser};
use repository::cargo::Cargo;

use super::{common::CommonOptions, heading, CargoCommand};

// roughly based on https://github.com/messense/cargo-options

/// Execute all unit and integration tests and build examples of a local package
#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
#[group(skip)]
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

    /// Test all packages in the workspace
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
    #[arg(long, help_heading = heading::PACKAGE_SELECTION)]
    pub all: bool,

    /// Test only this package's library
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub lib: bool,

    /// Test only the specified binary
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bin: Vec<String>,

    /// Test all binaries
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub bins: bool,

    /// Test only the specified example
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub example: Vec<String>,

    /// Test all examples
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub examples: bool,

    /// Test only the specified test target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub test: Vec<String>,

    /// Test all tests
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub tests: bool,

    /// Test only the specified bench target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bench: Vec<String>,

    /// Test all benches
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub benches: bool,

    /// Test all targets
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub all_targets: bool,

    /// Test only this library's documentation
    #[arg(long)]
    pub doc: bool,

    /// Compile, but don't run tests
    #[arg(long)]
    pub no_run: bool,

    /// Run all tests regardless of failure
    #[arg(long)]
    pub no_fail_fast: bool,

    /// Outputs a future incompatibility report at the end of the build (unstable)
    #[arg(long)]
    pub future_incompat_report: bool,

    /// If specified, only run tests containing this string in their names
    #[arg(value_name = "TESTNAME")]
    pub test_name: Option<String>,

    /// Arguments for the test binary
    #[arg(value_name = "args", trailing_var_arg = true, num_args = 0..)]
    pub args: Vec<String>,
}

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "test";

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
        if self.doc {
            cargo.arg("--doc");
        }
        if self.no_run {
            cargo.arg("--no-run");
        }
        if self.no_fail_fast {
            cargo.arg("--no-fail-fast");
        }
        if self.future_incompat_report {
            cargo.arg("--future-incompat-report");
        }

        if let Some(test_name) = self.test_name.as_ref() {
            cargo.arg(test_name);
        }

        if !self.args.is_empty() {
            cargo.arg("--");
            cargo.args(&self.args);
        }
    }

    fn profile(&self) -> &str {
        self.common.profile.as_deref().unwrap_or("test")
    }
}
