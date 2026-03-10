use clap::Parser;
use color_eyre::{
    eyre::{eyre, WrapErr as _},
    Result,
};
use hulkz_viewer::{setup_logging, ViewerApp, ViewerStartupOverrides};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "hulkz-viewer",
    version,
    about = "egui viewer for hulkz streams"
)]
struct Cli {
    /// Override initial namespace (default: persisted setting or empty/unset)
    #[arg(long, env = "HULKZ_NAMESPACE")]
    namespace: Option<String>,
    /// Override initial source path expression (e.g. "odometry", "/fleet/topic", "~node/private")
    #[arg(long)]
    source: Option<String>,
    /// Use persistent storage at the provided path instead of session temp storage
    #[arg(long)]
    storage_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    color_eyre::config::HookBuilder::new()
        .display_env_section(false)
        .install()
        .wrap_err("failed to install color-eyre hook")?;

    setup_logging().wrap_err("viewer startup failed while configuring logging")?;
    let startup_overrides = ViewerStartupOverrides {
        namespace: cli.namespace,
        source_expression: cli.source,
        storage_path: cli.storage_path,
    };

    eframe::run_native(
        "hulkz-viewer",
        eframe::NativeOptions::default(),
        Box::new(move |creation_context| {
            let app = ViewerApp::new(creation_context, startup_overrides.clone())
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|error| eyre!("failed to run eframe event loop: {error}"))?;

    Ok(())
}
