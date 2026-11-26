use clap::Args;
use repository::cargo::Environment;

#[derive(Args, Debug, Clone)]
pub struct EnvironmentArguments {
    /// Select the Cargo build environment
    #[arg(long, value_enum)]
    pub env: Option<Environment>,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long, visible_alias = "fern", visible_alias = "fernbedienung")]
    pub remote: bool,
}
