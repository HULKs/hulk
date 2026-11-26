use clap::{Args, ValueEnum};

/// Options for selecting the build environment and execution host.
#[derive(Args, Debug, Clone)]
pub struct EnvironmentArguments {
    /// Select the Cargo build environment
    #[arg(long, value_enum)]
    pub env: Option<Environment>,
    /// Run on a remote machine using scripts/remote_workspace
    #[arg(long, visible_alias = "fern", visible_alias = "fernbedienung")]
    pub remote: bool,
}

/// Environments in which Pepsi can run Cargo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Environment {
    /// Use Cargo from the execution host's PATH.
    PathCargo,
    /// Use the workstation Nix shell.
    #[value(alias = "native")]
    NixNative,
    /// Use the robot SDK Nix shell.
    #[value(alias = "nix", alias = "auto")]
    NixRobot,
}
