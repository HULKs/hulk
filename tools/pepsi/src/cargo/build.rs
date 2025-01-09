use std::path::PathBuf;
use std::process::Command;

use clap::{ArgAction, Parser};

use super::CargoCommand;
use super::{common::CommonOptions, heading};

#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
pub struct Arguments {
    #[command(flatten)]
    common: CommonOptions,

    /// Path to Cargo.toml
    #[arg(long, value_name = "PATH", help_heading = heading::MANIFEST_OPTIONS)]
    pub manifest_path: Option<PathBuf>,

    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'r', long, help_heading = heading::COMPILATION_OPTIONS)]
    release: bool,

    /// Ignore `rust-version` specification in packages
    #[arg(long)]
    ignore_rust_version: bool,

    /// Output build graph in JSON (unstable)
    #[arg(long, help_heading = heading::COMPILATION_OPTIONS)]
    unit_graph: bool,

    /// Package to build (see `cargo help pkgid`)
    #[arg(
        short = 'p',
        long = "package",
        value_name = "SPEC",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::PACKAGE_SELECTION,
    )]
    packages: Vec<String>,

    /// Build all packages in the workspace
    #[arg(long, help_heading = heading::PACKAGE_SELECTION)]
    workspace: bool,

    /// Exclude packages from the build
    #[arg(
        long,
        value_name = "SPEC",
        action = ArgAction::Append,
        help_heading = heading::PACKAGE_SELECTION,
    )]
    exclude: Vec<String>,

    /// Alias for workspace (deprecated)
    #[arg(long, help_heading = heading::PACKAGE_SELECTION)]
    all: bool,

    /// Build only this package's library
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    lib: bool,

    /// Build only the specified binary
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    bin: Vec<String>,

    /// Build all binaries
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    bins: bool,

    /// Build only the specified example
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    example: Vec<String>,

    /// Build all examples
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    examples: bool,

    /// Build only the specified test target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    test: Vec<String>,

    /// Build all tests
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    tests: bool,

    /// Build only the specified bench target
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        help_heading = heading::TARGET_SELECTION,
    )]
    bench: Vec<String>,

    /// Build all benches
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    benches: bool,

    /// Build all targets
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    all_targets: bool,

    /// Copy final artifacts to this directory (unstable)
    #[arg(long, alias = "out-dir", value_name = "PATH", help_heading = heading::COMPILATION_OPTIONS)]
    artifact_dir: Option<PathBuf>,

    /// Output the build plan in JSON (unstable)
    #[arg(long, help_heading = heading::COMPILATION_OPTIONS)]
    build_plan: bool,

    /// Outputs a future incompatibility report at the end of the build (unstable)
    #[arg(long)]
    future_incompat_report: bool,
}

impl CargoCommand for Arguments {
    fn apply(&self, cmd: &mut Command) {
        cmd.arg("build");

        self.common.apply(cmd);

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
        for pkg in &self.packages {
            cmd.arg("--package").arg(pkg);
        }
        if self.workspace {
            cmd.arg("--workspace");
        }
        for item in &self.exclude {
            cmd.arg("--exclude").arg(item);
        }
        if self.all {
            cmd.arg("--all");
        }
        if self.lib {
            cmd.arg("--lib");
        }
        for bin in &self.bin {
            cmd.arg("--bin").arg(bin);
        }
        if self.bins {
            cmd.arg("--bins");
        }
        for example in &self.example {
            cmd.arg("--example").arg(example);
        }
        if self.examples {
            cmd.arg("--examples");
        }
        for test in &self.test {
            cmd.arg("--test").arg(test);
        }
        if self.tests {
            cmd.arg("--tests");
        }
        for bench in &self.bench {
            cmd.arg("--bench").arg(bench);
        }
        if self.benches {
            cmd.arg("--benches");
        }
        if self.all_targets {
            cmd.arg("--all-targets");
        }
        if let Some(dir) = self.artifact_dir.as_ref() {
            cmd.arg("--artifact-dir").arg(dir);
        }
        if self.build_plan {
            cmd.arg("--build-plan");
        }
        if self.future_incompat_report {
            cmd.arg("--future-incompat-report");
        }
    }

    fn profile(&self) -> &str {
        self.common.profile.as_deref().unwrap_or("dev")
    }
}
