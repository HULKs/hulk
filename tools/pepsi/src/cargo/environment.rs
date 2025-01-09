use clap::Args;
use repository::cargo::Environment;

#[derive(Args, Debug)]
pub struct EnvironmentArguments {
    /// Use an SDK execution environment (default: native)
    #[arg(
        long,
        default_missing_value = "native",
        require_equals = true,
        num_args = 0..=1
    )]
    pub env: Option<Environment>,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}
