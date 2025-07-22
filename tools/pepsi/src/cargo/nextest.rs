use clap::{ArgAction, Parser};
use repository::cargo::Cargo;

use super::{heading, CargoCommand};

#[derive(Clone, Debug, Default, Parser)]
#[command(display_order = 1)]
pub struct Arguments {
    /// Build artifacts in release mode, with optimizations
    #[arg(short = 'r', long, help_heading = heading::COMPILATION_OPTIONS)]
    pub release: bool,

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

    /// Number of build/test jobs to run
    #[arg(short = 'j', long, help_heading = heading::COMPILATION_OPTIONS)]
    pub jobs: bool,

    /// Run tests with the specified Cargo profile
    #[arg(
        long,
        value_name = "PROFILE-NAME",
        help_heading = heading::COMPILATION_OPTIONS,
    )]
    pub profile: Option<String>,

    /// Test name filters
    #[arg(value_name = "args", trailing_var_arg = true, num_args = 0..)]
    pub args: Vec<String>,
}

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "nextest run";

    fn apply(&self, cargo: &mut Cargo) {
        if self.release {
            cargo.arg("--release");
        }
        if let Some(profile) = &self.profile {
            cargo.arg("--cargo-profile").arg(profile);
        }
        for pkg in &self.packages {
            cargo.arg("--package").arg(pkg);
        }
        if !self.args.is_empty() {
            cargo.arg("--");
            cargo.args(&self.args);
        }
    }

    fn profile(&self) -> &str {
        self.profile.as_deref().unwrap_or("dev")
    }
}
