use clap::Args;
use repository::cargo::SdkExecutor;

#[derive(Args, Debug, Clone)]
pub struct EnvironmentArguments {
    /// Use an SDK execution environment (default: installed)
    #[arg(
        long,
        default_missing_value = "installed",
        require_equals = true,
        num_args = 0..=1
    )]
    pub sdk: Option<SdkExecutor>,
    /// Use a remote machine for execution, see ./scripts/remote for details
    #[arg(long)]
    pub remote: bool,
}
