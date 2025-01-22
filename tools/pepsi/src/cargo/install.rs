use std::path::PathBuf;

use clap::{ArgAction, Parser};
use repository::cargo::Cargo;

use super::CargoCommand;
use super::{common::CommonOptions, heading};

// roughly based on https://github.com/messense/cargo-options

/// Install a Rust binary. Default location is $HOME/.cargo/bin
#[derive(Clone, Debug, Default, Parser)]
#[command(
    display_order = 1,
    after_help = "Run `cargo help install` for more detailed information."
)]
#[group(skip)]
pub struct Arguments {
    #[command(flatten)]
    pub common: CommonOptions,

    /// Specify a version to install
    #[arg(long, value_name = "VERSION", alias = "vers", requires = "crates")]
    pub version: Option<String>,

    /// Git URL to install the specified crate from
    #[arg(long, value_name = "URL", conflicts_with_all = ["index", "registry"])]
    pub git: Option<String>,

    /// Branch to use when installing from git
    #[arg(long, value_name = "BRANCH", requires = "git")]
    pub branch: Option<String>,

    /// Tag to use when installing from git
    #[arg(long, value_name = "TAG", requires = "git")]
    pub tag: Option<String>,

    /// Specific commit to use when installing from git
    #[arg(long, value_name = "SHA", requires = "git")]
    pub rev: Option<String>,

    /// list all installed packages and their versions
    #[arg(long)]
    pub list: bool,

    /// Force overwriting existing crates or binaries
    #[arg(short, long)]
    pub force: bool,

    /// Do not save tracking information
    #[arg(long)]
    pub no_track: bool,

    /// Build in debug mode (with the 'dev' profile) instead of release mode
    #[arg(long)]
    pub debug: bool,

    /// Directory to install packages into
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Registry index to install from
    #[arg(
        long,
        value_name = "INDEX",
        conflicts_with_all = ["git", "registry"],
        requires = "crates",
    )]
    pub index: Option<String>,

    /// Registry to use
    #[arg(
        long,
        value_name = "REGISTRY",
        conflicts_with_all = ["git", "index"],
        requires = "crates",
    )]
    pub registry: Option<String>,

    /// Install only the specified binary
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub bin: Vec<String>,

    /// Install all binaries
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub bins: bool,

    /// Install only the specified example
    #[arg(
        long,
        value_name = "NAME",
        action = ArgAction::Append,
        num_args=0..=1,
        help_heading = heading::TARGET_SELECTION,
    )]
    pub example: Vec<String>,

    /// Install all examples
    #[arg(long, help_heading = heading::TARGET_SELECTION)]
    pub examples: bool,

    #[arg(value_name = "crate", action = ArgAction::Append, num_args = 0..)]
    pub crates: Vec<String>,
}

impl CargoCommand for Arguments {
    const SUB_COMMAND: &'static str = "install";

    fn apply(&self, cargo: &mut Cargo) {
        self.common.apply(cargo);

        if let Some(version) = self.version.as_ref() {
            cargo.arg("--version").arg(version);
        }
        if let Some(git) = self.git.as_ref() {
            cargo.arg("--git").arg(git);
        }
        if let Some(branch) = self.branch.as_ref() {
            cargo.arg("--branch").arg(branch);
        }
        if let Some(tag) = self.tag.as_ref() {
            cargo.arg("--tag").arg(tag);
        }
        if let Some(rev) = self.rev.as_ref() {
            cargo.arg("--rev").arg(rev);
        }
        if self.list {
            cargo.arg("--list");
        }
        if self.force {
            cargo.arg("--force");
        }
        if self.no_track {
            cargo.arg("--no-track");
        }
        if self.debug {
            cargo.arg("--debug");
        }
        if let Some(root) = self.root.as_ref() {
            cargo.arg("--root").arg(root);
        }
        if let Some(index) = self.index.as_ref() {
            cargo.arg("--index").arg(index);
        }
        if let Some(registry) = self.registry.as_ref() {
            cargo.arg("--registry").arg(registry);
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
        cargo.args(&self.crates);
    }

    fn profile(&self) -> &str {
        self.common.profile.as_deref().unwrap_or("release")
    }
}
